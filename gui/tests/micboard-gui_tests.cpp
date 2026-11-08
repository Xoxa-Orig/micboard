#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QTemporaryDir>
#include <QtTest>

#include "backend.h"
#include "theme.h"
#include "translator.h"

class MicBoardGuiTests : public QObject {
    Q_OBJECT

private slots:
    void themeFallsBackWithoutHyprland()
    {
        QTemporaryDir dir;
        Theme theme(dir.path() + "/missing-hyprctl");
        QCOMPARE(theme.radius(), 0);
    }

    void themeReadsHyprlandRounding()
    {
        QTemporaryDir dir;
        const QString response = dir.filePath("response.json");
        const QString command = dir.filePath("hyprctl");
        QVERIFY(write(command, "#!/bin/sh\ncat '" + response.toUtf8() + "'\n"));
        QVERIFY(QFile::setPermissions(command, QFile::ReadOwner | QFile::WriteOwner
                                                   | QFile::ExeOwner));
        QVERIFY(write(response, "{\"int\":6}"));

        Theme theme(command);
        QTRY_COMPARE(theme.radius(), 6);
    }

    void themeSwitchesToDark()
    {
        Theme theme;
        QCOMPARE(theme.darkMode(), false);
        QCOMPARE(theme.bg(), QString("#F2EDE1"));
        QCOMPARE(theme.ink(), QString("#24261F"));

        QSignalSpy changed(&theme, &Theme::changed);
        theme.setDarkMode(true);
        QCOMPARE(changed.count(), 1);
        QCOMPARE(theme.darkMode(), true);
        QCOMPARE(theme.bg(), QString("#121A16"));
        QCOMPARE(theme.ink(), QString("#EFE8D8"));
        QCOMPARE(theme.panel(), QString("#1A241F"));
        QCOMPARE(theme.dangerSoft(), QString("#3B2621"));

        theme.toggleDarkMode();
        QCOMPARE(theme.darkMode(), false);
        QCOMPARE(theme.bg(), QString("#F2EDE1"));
    }

    void themeFollowsTheOmarchyMode()
    {
        QTemporaryDir dir;
        QVERIFY(QDir(dir.path()).mkpath("omarchy/current/theme"));
        const QString colors = dir.filePath("omarchy/current/theme/colors.toml");

        const QByteArray oldState = qgetenv("XDG_STATE_HOME");
        qputenv("XDG_STATE_HOME", dir.path().toUtf8());

        QVERIFY(write(colors, "mode = \"dark\"\nbackground = \"#0d1420\"\n"));
        QCOMPARE(Theme::omarchyPrefersDark(), true);

        QVERIFY(write(colors, "# a comment about the mode\nmode = \"light\"\n"));
        QCOMPARE(Theme::omarchyPrefersDark(), false);

        QFile::remove(colors);
        QCOMPARE(Theme::omarchyPrefersDark(), false);

        if (oldState.isEmpty())
            qunsetenv("XDG_STATE_HOME");
        else
            qputenv("XDG_STATE_HOME", oldState);
    }

    void translatorFallsBackToEnglish()
    {
        QTemporaryDir dir;
        Translator translator(dir.path() + "/language", dir.path());
        QCOMPARE(translator.language(), QString("en"));
        QVERIFY(translator.translate("Main", "Save").isNull());
    }

    void translatorFollowsTheLanguageFile()
    {
        QTemporaryDir dir;
        QVERIFY(write(dir.path() + "/language", "pt-BR\n"));
        QVERIFY(write(dir.path() + "/pt.json", R"({"Save": "Guardar", "Open": "Abrir"})"));
        QVERIFY(write(dir.path() + "/pt-BR.json", R"({"Save": "Salvar"})"));

        Translator translator(dir.path() + "/language", dir.path());
        QCOMPARE(translator.language(), QString("pt-BR"));
        QCOMPARE(translator.translate("Main", "Save"), QString("Salvar"));
        QCOMPARE(translator.translate("Main", "Open"), QString("Abrir"));
        QVERIFY(translator.translate("Main", "Quit").isNull());
    }

    void translatorPicksRussianPlurals()
    {
        QTemporaryDir dir;
        QVERIFY(write(dir.path() + "/language", "ru\n"));
        QVERIFY(write(dir.path() + "/en.json",
                      R"({"%n sounds": {"one": "%n sound", "other": "%n sounds"}})"));
        QVERIFY(write(dir.path() + "/ru.json",
                      R"({"%n sounds": {"one": "%n звук", "few": "%n звука", "many": "%n звуков", "other": "%n звука"}})"));

        Translator translator(dir.path() + "/language", dir.path());
        QCOMPARE(translator.language(), QString("ru"));
        QCOMPARE(translator.translate("Main", "%n sounds", nullptr, 1), QString("%n звук"));
        QCOMPARE(translator.translate("Main", "%n sounds", nullptr, 2), QString("%n звука"));
        QCOMPARE(translator.translate("Main", "%n sounds", nullptr, 5), QString("%n звуков"));
        QCOMPARE(Translator::pluralCategory("ar", 11), QString("many"));
        QCOMPARE(Translator::pluralCategory("ja", 1), QString("other"));
    }

    void backendReadsTheCliJson()
    {
        QTemporaryDir dir;
        const QString command = dir.filePath("micboard");
        QVERIFY(write(command, R"(#!/bin/sh
if [ "$1" = "list" ]; then
cat <<'EOF'
{"config":"/tmp/x.toml","device":"MicBoard","master_volume":0.8,"mode":"overlap","hotkeys":"auto","virtual_mic":{"source":"MicBoard_Mic"},"sounds":[{"id":"tone","file":"/tmp/tone.wav","hotkey":"ctrl+alt+1","volume":1.0,"exists":true}]}
EOF
elif [ "$1" = "status" ]; then
echo '{"running":true,"playing":1,"active":["tone"]}'
fi
)"));
        QVERIFY(QFile::setPermissions(command, QFile::ReadOwner | QFile::WriteOwner
                                                   | QFile::ExeOwner));

        const QByteArray oldBinary = qgetenv("MICBOARD_BIN");
        const QByteArray oldConfig = qgetenv("MICBOARD_CONFIG");
        qputenv("MICBOARD_BIN", command.toUtf8());
        qputenv("MICBOARD_CONFIG", (dir.path() + "/config.toml").toUtf8());

        Backend backend;
        QCOMPARE(backend.soundCount(), 1);
        QCOMPARE(backend.running(), true);
        QCOMPARE(backend.device(), QString("MicBoard"));
        QCOMPARE(backend.source(), QString("MicBoard_Mic"));
        QCOMPARE(backend.mode(), QString("overlap"));
        QVERIFY(qAbs(backend.masterVolume() - 0.8) < 0.001);
        QCOMPARE(backend.sounds().first().toMap().value("id").toString(), QString("tone"));
        QCOMPARE(backend.playing(), 1);
        QCOMPARE(backend.playingIds(), QStringList{"tone"});

        if (oldBinary.isEmpty())
            qunsetenv("MICBOARD_BIN");
        else
            qputenv("MICBOARD_BIN", oldBinary);
        if (oldConfig.isEmpty())
            qunsetenv("MICBOARD_CONFIG");
        else
            qputenv("MICBOARD_CONFIG", oldConfig);
    }

    void backendStopsOneSound()
    {
        QTemporaryDir dir;
        const QString command = dir.filePath("micboard");
        const QString log = dir.filePath("stopped.log");
        QVERIFY(write(command, R"(#!/bin/sh
case "$1" in
  list) echo '{"config":"/tmp/x.toml","device":"MicBoard","master_volume":1.0,"mode":"overlap","hotkeys":"auto","virtual_mic":{"source":"MicBoard_Mic"},"sounds":[]}' ;;
  status) echo '{"running":true,"playing":2,"active":["tone","horn"]}' ;;
  stop) echo "$2" >> "$FAKE_LOG" ;;
esac
)"));
        QVERIFY(QFile::setPermissions(command, QFile::ReadOwner | QFile::WriteOwner
                                                   | QFile::ExeOwner));

        const QByteArray oldBinary = qgetenv("MICBOARD_BIN");
        const QByteArray oldConfig = qgetenv("MICBOARD_CONFIG");
        const QByteArray oldLog = qgetenv("FAKE_LOG");
        qputenv("MICBOARD_BIN", command.toUtf8());
        qputenv("MICBOARD_CONFIG", (dir.path() + "/config.toml").toUtf8());
        qputenv("FAKE_LOG", log.toUtf8());

        Backend backend;
        QVERIFY(backend.running());
        QCOMPARE(backend.playingIds(), (QStringList{"tone", "horn"}));

        backend.stopSound("tone");
        QTRY_VERIFY_WITH_TIMEOUT(QFileInfo(log).size() > 0, 4000);

        QFile stopped(log);
        QVERIFY(stopped.open(QIODevice::ReadOnly));
        QCOMPARE(QString::fromUtf8(stopped.readAll()).trimmed(), QString("tone"));

        if (oldBinary.isEmpty())
            qunsetenv("MICBOARD_BIN");
        else
            qputenv("MICBOARD_BIN", oldBinary);
        if (oldConfig.isEmpty())
            qunsetenv("MICBOARD_CONFIG");
        else
            qputenv("MICBOARD_CONFIG", oldConfig);
        if (oldLog.isEmpty())
            qunsetenv("FAKE_LOG");
        else
            qputenv("FAKE_LOG", oldLog);
    }

    void backendStartsTheDaemonBeforePlaying()
    {
        QTemporaryDir dir;
        const QString command = dir.filePath("micboard");
        const QString state = dir.filePath("state");
        const QString log = dir.filePath("played.log");
        QVERIFY(write(command, R"(#!/bin/sh
case "$1" in
  list) echo '{"config":"/tmp/x.toml","device":"MicBoard","master_volume":1.0,"mode":"overlap","hotkeys":"auto","virtual_mic":{"source":"MicBoard_Mic"},"sounds":[]}' ;;
  status) if [ -f "$FAKE_STATE" ]; then echo '{"running":true,"playing":0}'; else echo '{"running":false}'; fi ;;
  run) touch "$FAKE_STATE" ;;
  play) echo "$2" >> "$FAKE_LOG" ;;
  quit) rm -f "$FAKE_STATE" ;;
esac
)"));
        QVERIFY(QFile::setPermissions(command, QFile::ReadOwner | QFile::WriteOwner
                                                   | QFile::ExeOwner));

        const QByteArray oldBinary = qgetenv("MICBOARD_BIN");
        const QByteArray oldConfig = qgetenv("MICBOARD_CONFIG");
        qputenv("MICBOARD_BIN", command.toUtf8());
        qputenv("MICBOARD_CONFIG", (dir.path() + "/config.toml").toUtf8());
        qputenv("FAKE_STATE", state.toUtf8());
        qputenv("FAKE_LOG", log.toUtf8());

        Backend backend;
        QVERIFY(!backend.running());
        backend.play("tone");
        QTRY_VERIFY_WITH_TIMEOUT(backend.running(), 6000);
        QTRY_VERIFY_WITH_TIMEOUT(QFileInfo(log).size() > 0, 6000);

        QFile played(log);
        QVERIFY(played.open(QIODevice::ReadOnly));
        QCOMPARE(QString::fromUtf8(played.readAll()).trimmed(), QString("tone"));

        qunsetenv("FAKE_STATE");
        qunsetenv("FAKE_LOG");
        if (oldBinary.isEmpty())
            qunsetenv("MICBOARD_BIN");
        else
            qputenv("MICBOARD_BIN", oldBinary);
        if (oldConfig.isEmpty())
            qunsetenv("MICBOARD_CONFIG");
        else
            qputenv("MICBOARD_CONFIG", oldConfig);
    }

private:
    static bool write(const QString &path, const QByteArray &contents)
    {
        QFile file(path);
        return file.open(QIODevice::WriteOnly) && file.write(contents) == contents.size();
    }
};

QTEST_MAIN(MicBoardGuiTests)
#include "micboard-gui_tests.moc"
