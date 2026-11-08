#include <QDir>
#include <QFile>
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
echo '{"running":true,"playing":0}'
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
