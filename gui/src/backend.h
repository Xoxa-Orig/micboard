#pragma once

#include <QDateTime>
#include <QObject>
#include <QProcess>
#include <QStringList>
#include <QTimer>
#include <QVariantList>
#include <QVariantMap>

// The window's only job is to drive the existing `micboard` daemon: list the
// configured sounds, start them, stop them, and keep the config in sync. All
// of that lives behind the CLI's JSON output, so the GUI never parses TOML.
class Backend : public QObject {
    Q_OBJECT
    Q_PROPERTY(QVariantList sounds READ sounds NOTIFY soundsChanged)
    Q_PROPERTY(int soundCount READ soundCount NOTIFY soundsChanged)
    Q_PROPERTY(bool running READ running NOTIFY runningChanged)
    Q_PROPERTY(bool starting READ starting NOTIFY startingChanged)
    Q_PROPERTY(int playing READ playing NOTIFY playingChanged)
    Q_PROPERTY(double masterVolume READ masterVolume WRITE setMasterVolume NOTIFY masterVolumeChanged)
    Q_PROPERTY(QString device READ device NOTIFY configChanged)
    Q_PROPERTY(QString source READ source NOTIFY configChanged)
    Q_PROPERTY(QString mode READ mode NOTIFY configChanged)
    Q_PROPERTY(QString hotkeys READ hotkeys NOTIFY configChanged)
    Q_PROPERTY(QString configPath READ configPath NOTIFY configChanged)
    Q_PROPERTY(QString message READ message NOTIFY messageChanged)
    Q_PROPERTY(bool messageIsError READ messageIsError NOTIFY messageChanged)
    Q_PROPERTY(QString lastPlayedId READ lastPlayedId NOTIFY lastPlayedIdChanged)
    Q_PROPERTY(QString binPath READ binPath CONSTANT)
    Q_PROPERTY(bool available READ available NOTIFY configChanged)

public:
    explicit Backend(QObject *parent = nullptr);

    QVariantList sounds() const { return m_sounds; }
    int soundCount() const { return m_sounds.size(); }
    bool running() const { return m_running; }
    bool starting() const { return m_starting; }
    int playing() const { return m_playing; }
    double masterVolume() const { return m_masterVolume; }
    QString device() const { return m_device; }
    QString source() const { return m_source; }
    QString mode() const { return m_mode; }
    QString hotkeys() const { return m_hotkeys; }
    QString configPath() const { return m_configPath; }
    QString message() const { return m_message; }
    bool messageIsError() const { return m_messageIsError; }
    QString lastPlayedId() const { return m_lastPlayedId; }
    QString binPath() const { return m_bin; }
    bool available() const { return m_available; }

    Q_INVOKABLE void refresh();
    Q_INVOKABLE void play(const QString &id);
    Q_INVOKABLE void stopAll();
    Q_INVOKABLE void startDaemon();
    Q_INVOKABLE void stopDaemon();
    Q_INVOKABLE void reloadConfig();
    Q_INVOKABLE void addSound(const QString &path);
    Q_INVOKABLE void addDropped(const QVariantList &urls);
    Q_INVOKABLE void removeSound(const QString &id);
    Q_INVOKABLE void chooseFile();
    Q_INVOKABLE void openConfig();
    Q_INVOKABLE void clearMessage();
    Q_INVOKABLE void setMasterVolume(double volume);

signals:
    void soundsChanged();
    void runningChanged();
    void startingChanged();
    void playingChanged();
    void masterVolumeChanged();
    void configChanged();
    void messageChanged();
    void lastPlayedIdChanged();

private slots:
    void poll();
    void onPortalResponse(uint response, const QVariantMap &results);

private:
    QByteArray runSync(const QStringList &arguments, int *exitCode = nullptr) const;
    void runAction(const QStringList &arguments, const QString &success, bool reloadConfig);
    void loadConfig();
    void loadStatus();
    void setMessage(const QString &message, bool error = false);
    QString findBinary() const;
    QString defaultConfigPath() const;

    QString m_bin;
    QString m_configPath;
    bool m_available = false;

    QVariantList m_sounds;
    bool m_running = false;
    bool m_starting = false;
    int m_playing = 0;
    double m_masterVolume = 1.0;
    bool m_volumeDirty = false;
    QString m_device;
    QString m_source;
    QString m_mode = "overlap";
    QString m_hotkeys = "auto";
    QString m_message;
    bool m_messageIsError = false;
    QString m_lastPlayedId;
    qint64 m_startDeadline = 0;

    QTimer m_poll;
    QTimer m_volumeDebounce;
    int m_ticks = 0;
};
