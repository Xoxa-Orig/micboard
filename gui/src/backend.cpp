#include "backend.h"

#include <QCoreApplication>
#include <QDBusConnection>
#include <QDBusMessage>
#include <QDBusPendingCall>
#include <QDBusPendingCallWatcher>
#include <QDesktopServices>
#include <QDir>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QStandardPaths>
#include <QUrl>

Backend::Backend(QObject *parent) : QObject(parent) {
    m_bin = findBinary();
    m_configPath = defaultConfigPath();

    m_volumeDebounce.setSingleShot(true);
    m_volumeDebounce.setInterval(180);
    connect(&m_volumeDebounce, &QTimer::timeout, this, [this] {
        m_volumeDirty = false;
        if (!m_bin.isEmpty())
            runAction({"volume", QString::number(m_masterVolume, 'f', 2)}, {}, false);
    });

    // Idle polling is cheap; while something is audible the window switches
    // to a faster interval so the playing indicators stay honest.
    m_poll.setInterval(1200);
    connect(&m_poll, &QTimer::timeout, this, &Backend::poll);
    m_poll.start();

    refresh();
}

QString Backend::findBinary() const {
    const QString configured = qEnvironmentVariable("MICBOARD_BIN");
    if (!configured.isEmpty() && QFileInfo(configured).isExecutable())
        return configured;

    const QString found = QStandardPaths::findExecutable("micboard");
    if (!found.isEmpty())
        return found;

    const QString app = QCoreApplication::applicationDirPath();
    const QStringList candidates{
        app + "/micboard",
        app + "/../target/release/micboard",
        app + "/../../target/release/micboard",
        QDir::homePath() + "/.local/bin/micboard",
        "/usr/bin/micboard",
    };
    for (const auto &candidate : candidates)
        if (QFileInfo(candidate).isExecutable())
            return candidate;
    return {};
}

QString Backend::defaultConfigPath() const {
    const QString configured = qEnvironmentVariable("MICBOARD_CONFIG");
    if (!configured.isEmpty())
        return configured;
    return QStandardPaths::writableLocation(QStandardPaths::GenericConfigLocation)
           + "/micboard/config.toml";
}

QByteArray Backend::runSync(const QStringList &arguments, int *exitCode) const {
    if (exitCode)
        *exitCode = -1;
    if (m_bin.isEmpty())
        return {};

    QProcess process;
    process.start(m_bin, arguments);
    if (!process.waitForStarted(1500))
        return {};
    if (!process.waitForFinished(4000)) {
        process.kill();
        process.waitForFinished(500);
        return {};
    }
    if (exitCode)
        *exitCode = process.exitCode();
    return process.readAllStandardOutput();
}

void Backend::refresh() {
    m_bin = findBinary();
    m_available = !m_bin.isEmpty();
    if (!m_available) {
        setMessage(tr("MicBoard is not installed or not in PATH."), true);
        emit configChanged();
        return;
    }
    loadConfig();
    loadStatus();
}

void Backend::loadConfig() {
    int code = -1;
    const QByteArray output = runSync({"list", "--json"}, &code);
    if (code != 0) {
        m_available = false;
        setMessage(tr("Cannot read the MicBoard config."), true);
        emit configChanged();
        return;
    }

    QJsonParseError error;
    const QJsonDocument document = QJsonDocument::fromJson(output, &error);
    if (error.error != QJsonParseError::NoError || !document.isObject()) {
        setMessage(tr("Cannot read the MicBoard config."), true);
        return;
    }

    m_available = true;
    const QJsonObject root = document.object();

    const QString configPath = root.value("config").toString();
    if (!configPath.isEmpty() && configPath != m_configPath) {
        m_configPath = configPath;
        emit configChanged();
    }

    QString device = root.value("device").toString();
    if (device.isEmpty())
        device = tr("System default");
    if (device != m_device) {
        m_device = device;
        emit configChanged();
    }

    const QJsonObject virtualMic = root.value("virtual_mic").toObject();
    const QString source = virtualMic.value("source").toString();
    if (source != m_source) {
        m_source = source;
        emit configChanged();
    }

    const QString mode = root.value("mode").toString();
    if (!mode.isEmpty() && mode != m_mode) {
        m_mode = mode;
        emit configChanged();
    }

    const QString hotkeys = root.value("hotkeys").toString();
    if (!hotkeys.isEmpty() && hotkeys != m_hotkeys) {
        m_hotkeys = hotkeys;
        emit configChanged();
    }

    const double volume = root.value("master_volume").toDouble(1.0);
    if (!m_volumeDirty && qAbs(volume - m_masterVolume) > 0.001) {
        m_masterVolume = volume;
        emit masterVolumeChanged();
    }

    QVariantList sounds;
    const QJsonArray array = root.value("sounds").toArray();
    for (const auto &value : array) {
        const QJsonObject sound = value.toObject();
        const QString file = sound.value("file").toString();
        QVariantMap entry;
        entry.insert("id", sound.value("id").toString());
        entry.insert("file", file);
        entry.insert("name", QFileInfo(file).completeBaseName());
        entry.insert("hotkey", sound.value("hotkey").toString());
        entry.insert("volume", sound.value("volume").toDouble(1.0));
        entry.insert("exists", sound.value("exists").toBool());
        sounds.append(entry);
    }

    if (sounds != m_sounds) {
        m_sounds = sounds;
        emit soundsChanged();
    }
}

void Backend::loadStatus() {
    if (m_bin.isEmpty())
        return;

    const QByteArray output = runSync({"status", "--json"});
    const QJsonDocument document = QJsonDocument::fromJson(output);
    if (!document.isObject())
        return;

    const QJsonObject root = document.object();
    const bool running = root.value("running").toBool();
    if (running != m_running) {
        m_running = running;
        emit runningChanged();
    }

    if (running) {
        const int playing = root.value("playing").toInt();
        QStringList playingIds;
        for (const auto &value : root.value("active").toArray())
            playingIds.append(value.toString());
        if (playing != m_playing || playingIds != m_playingIds) {
            m_playing = playing;
            m_playingIds = playingIds;
            emit playingChanged();
        }
        if (playing == 0 && !m_lastPlayedId.isEmpty()) {
            m_lastPlayedId.clear();
            emit lastPlayedIdChanged();
        }
        if (m_starting) {
            m_starting = false;
            emit startingChanged();
        }
        if (!m_pendingPlay.isEmpty()) {
            const QString id = m_pendingPlay;
            m_pendingPlay.clear();
            play(id);
        }
    } else {
        if (m_playing != 0 || !m_playingIds.isEmpty()) {
            m_playing = 0;
            m_playingIds.clear();
            emit playingChanged();
        }
        if (m_starting && QDateTime::currentMSecsSinceEpoch() > m_startDeadline) {
            m_starting = false;
            m_pendingPlay.clear();
            emit startingChanged();
            setMessage(tr("Could not start the daemon."), true);
        }
    }
}

void Backend::poll() {
    if (!m_available)
        return;
    loadStatus();
    m_poll.setInterval(m_playing > 0 ? 400 : 1200);
    if (++m_ticks % 3 == 0)
        loadConfig();
}

void Backend::runAction(const QStringList &arguments, const QString &success,
                        bool reloadConfig) {
    if (m_bin.isEmpty()) {
        setMessage(tr("MicBoard is not installed or not in PATH."), true);
        return;
    }

    auto *process = new QProcess(this);
    connect(process, &QProcess::errorOccurred, this,
            [this, process](QProcess::ProcessError error) {
                if (error != QProcess::FailedToStart)
                    return;
                setMessage(tr("Cannot run MicBoard."), true);
                process->deleteLater();
            });
    connect(process, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished), this,
            [this, process, success, reloadConfig](int code, QProcess::ExitStatus status) {
                const QString error =
                    QString::fromUtf8(process->readAllStandardError()).trimmed();
                if (code != 0 || status != QProcess::NormalExit) {
                    setMessage(error.isEmpty() ? tr("The command failed.") : error, true);
                } else if (!success.isEmpty()) {
                    setMessage(success, false);
                }
                process->deleteLater();
                if (reloadConfig)
                    loadConfig();
                else
                    loadStatus();
            });
    process->start(m_bin, arguments);
}

void Backend::play(const QString &id) {
    // Clicking a card while the daemon is offline starts it and plays the
    // sound as soon as the control port answers, so the window never feels
    // like it is refusing to work.
    if (!m_running) {
        m_pendingPlay = id;
        startDaemon();
        return;
    }
    if (!m_lastPlayedId.isEmpty() && m_lastPlayedId != id) {
        m_lastPlayedId.clear();
    }
    m_lastPlayedId = id;
    emit lastPlayedIdChanged();
    runAction({"play", id}, {}, false);
    QTimer::singleShot(150, this, &Backend::poll);
}

void Backend::stopSound(const QString &id) {
    if (!m_running)
        return;
    if (m_lastPlayedId == id) {
        m_lastPlayedId.clear();
        emit lastPlayedIdChanged();
    }
    runAction({"stop", id}, {}, false);
    QTimer::singleShot(120, this, &Backend::poll);
}

void Backend::stopAll() {
    if (!m_running)
        return;
    m_lastPlayedId.clear();
    emit lastPlayedIdChanged();
    runAction({"stop"}, tr("Stopped all sounds."), false);
    QTimer::singleShot(120, this, &Backend::poll);
}

void Backend::startDaemon() {
    if (m_running || m_starting)
        return;
    if (m_bin.isEmpty()) {
        setMessage(tr("MicBoard is not installed or not in PATH."), true);
        return;
    }

    m_starting = true;
    m_startDeadline = QDateTime::currentMSecsSinceEpoch() + 12000;
    emit startingChanged();

    if (!QProcess::startDetached(m_bin, {"run"}, QDir::homePath())) {
        m_starting = false;
        m_pendingPlay.clear();
        emit startingChanged();
        setMessage(tr("Could not start the daemon."), true);
        return;
    }

    setMessage(tr("Starting the daemon…"), false);
    QTimer::singleShot(700, this, &Backend::poll);
}

void Backend::stopDaemon() {
    m_pendingPlay.clear();
    runAction({"quit"}, tr("Daemon stopped."), false);
}

void Backend::reloadConfig() {
    runAction({"reload"}, tr("Config reloaded."), true);
}

void Backend::addSound(const QString &path) {
    const QFileInfo info(path);
    if (!info.isFile()) {
        setMessage(tr("File not found: %1").arg(path), true);
        return;
    }
    runAction({"add", path}, tr("Added \"%1\".").arg(info.completeBaseName()), true);
}

void Backend::addDropped(const QVariantList &urls) {
    for (const auto &value : urls) {
        const QString path = value.toUrl().toLocalFile();
        if (!path.isEmpty())
            addSound(path);
    }
}

void Backend::removeSound(const QString &id) {
    runAction({"remove", id}, tr("Removed \"%1\".").arg(id), true);
}

void Backend::chooseFile() {
    if (!QDBusConnection::sessionBus().isConnected()) {
        setMessage(tr("File picker is unavailable — drag and drop a file instead."), true);
        return;
    }

    const QString token = "micboard" + QString::number(QDateTime::currentMSecsSinceEpoch());
    QString sender = QDBusConnection::sessionBus().baseService();
    sender.remove(':');
    sender.replace('.', '_');
    const QString predicted =
        "/org/freedesktop/portal/desktop/request/" + sender + "/" + token;

    QDBusConnection::sessionBus().connect(
        "org.freedesktop.portal.Desktop", predicted, "org.freedesktop.portal.Request",
        "Response", this, SLOT(onPortalResponse(uint,QVariantMap)));

    QVariantMap options;
    options.insert("handle_token", token);
    options.insert("multiple", false);
    options.insert("accept_label", tr("Add"));

    QDBusMessage message = QDBusMessage::createMethodCall(
        "org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.FileChooser", "OpenFile");
    message << QString() << tr("Add a sound") << options;

    auto *watcher = new QDBusPendingCallWatcher(
        QDBusConnection::sessionBus().asyncCall(message), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, watcher, predicted](QDBusPendingCallWatcher *call) {
                call->deleteLater();
                if (call->isError()) {
                    setMessage(tr("File picker is unavailable — drag and drop a file instead."),
                               true);
                    return;
                }
                const QString handle = call->reply().arguments().value(0).toString();
                if (!handle.isEmpty() && handle != predicted) {
                    QDBusConnection::sessionBus().connect(
                        "org.freedesktop.portal.Desktop", handle,
                        "org.freedesktop.portal.Request", "Response", this,
                        SLOT(onPortalResponse(uint,QVariantMap)));
                }
            });
}

void Backend::onPortalResponse(uint response, const QVariantMap &results) {
    if (response != 0)
        return;
    const QStringList uris = results.value("uris").toStringList();
    if (uris.isEmpty())
        return;
    const QString path = QUrl(uris.first()).toLocalFile();
    if (!path.isEmpty())
        addSound(path);
}

void Backend::openConfig() {
    if (!QDesktopServices::openUrl(QUrl::fromLocalFile(m_configPath)))
        setMessage(tr("Could not open the config file."), true);
}

void Backend::setMasterVolume(double volume) {
    volume = qBound(0.0, volume, 1.5);
    if (qAbs(volume - m_masterVolume) < 0.001)
        return;
    m_masterVolume = volume;
    m_volumeDirty = true;
    emit masterVolumeChanged();
    m_volumeDebounce.start();
}

void Backend::clearMessage() {
    if (m_message.isEmpty())
        return;
    m_message.clear();
    m_messageIsError = false;
    emit messageChanged();
}

void Backend::setMessage(const QString &message, bool error) {
    m_message = message;
    m_messageIsError = error;
    emit messageChanged();
}
