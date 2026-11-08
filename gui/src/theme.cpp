#include "theme.h"

#include <QDir>
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QStandardPaths>

Theme::Theme(const QString &hyprctl, QObject *parent)
    : QObject(parent),
      m_hyprctl(!hyprctl.isEmpty()
                    ? hyprctl
                    : !qEnvironmentVariableIsEmpty("HYPRLAND_INSTANCE_SIGNATURE")
                          ? QStandardPaths::findExecutable("hyprctl")
                          : QString()) {
    m_timeout.setSingleShot(true);
    m_timeout.setInterval(500);
    connect(&m_timeout, &QTimer::timeout, &m_process, &QProcess::kill);
    connect(&m_process, &QProcess::errorOccurred, this, [this] { m_timeout.stop(); });
    connect(&m_process, &QProcess::finished, this,
            [this](int code, QProcess::ExitStatus status) {
                m_timeout.stop();
                const auto value = QJsonDocument::fromJson(m_process.readAllStandardOutput())
                                       .object()
                                       .value("int");
                if (code != 0 || status != QProcess::NormalExit || !value.isDouble())
                    return;
                if (value.toInt() != m_radius) {
                    m_radius = value.toInt();
                    emit changed();
                }
            });

    m_poll.setInterval(1500);
    connect(&m_poll, &QTimer::timeout, this, &Theme::refreshRounding);
    if (!m_hyprctl.isEmpty()) {
        refreshRounding();
        m_poll.start();
    }
}

Theme::~Theme() {
    if (m_process.state() != QProcess::NotRunning) {
        m_process.kill();
        m_process.waitForFinished(1000);
    }
}

void Theme::setDarkMode(bool dark) {
    if (dark == m_dark)
        return;
    m_dark = dark;
    emit darkModeChanged();
    emit changed();
}

bool Theme::omarchyPrefersDark() {
    const QString state = qEnvironmentVariable(
        "XDG_STATE_HOME", QDir::homePath() + "/.local/state");
    QFile file(state + "/omarchy/current/theme/colors.toml");
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text))
        return false;
    while (!file.atEnd()) {
        const QString line = QString::fromUtf8(file.readLine()).trimmed();
        if (line.startsWith('#'))
            continue;
        if (line.section('=', 0, 0).trimmed() != "mode")
            continue;
        const QString value = line.section('=', 1).trimmed();
        return value.contains("dark", Qt::CaseInsensitive);
    }
    return false;
}

void Theme::refreshRounding() {
    if (m_hyprctl.isEmpty() || m_process.state() != QProcess::NotRunning)
        return;
    m_process.start(m_hyprctl, {"-j", "getoption", "decoration:rounding"});
    m_timeout.start();
}
