// MicBoard's window: a small, keyboard-friendly front end for the daemon.

#include <QGuiApplication>
#include <QIcon>
#include <QLocale>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>
#include <QQuickWindow>
#include <QSettings>
#include <QTimer>
#include <QUrl>

#include "backend.h"
#include "theme.h"
#include "translator.h"

int main(int argc, char *argv[]) {
    QGuiApplication app(argc, argv);
    app.setApplicationName("micboard");
    app.setApplicationVersion("0.5.0");

    // Matches the window to micboard.desktop, so the compositor (Wayland
    // app_id) and the launcher pick up the installed icon.
    app.setDesktopFileName("micboard");
    app.setWindowIcon(QIcon::fromTheme("micboard"));

    // The window draws its own controls, so the Basic style keeps popups and
    // scrollbars out of Material's way.
    QQuickStyle::setStyle("Basic");

    QString language = qEnvironmentVariable("MICBOARD_LANGUAGE");
    if (language.isEmpty())
        language = QLocale::system().name().section('_', 0, 0);
    if (language.isEmpty() || language == "C")
        language = "en";

    Translator translator({}, ":/i18n", nullptr, language);
    app.installTranslator(&translator);

    const QStringList arguments = QCoreApplication::arguments();
    QSettings settings("micboard", "micboard-gui");
    bool dark = settings.contains("darkMode")
                    ? settings.value("darkMode").toBool()
                    : Theme::omarchyPrefersDark();
    if (arguments.contains("--dark"))
        dark = true;
    if (arguments.contains("--light"))
        dark = false;

    Theme theme;
    theme.setDarkMode(dark);
    // Only an explicit toggle is remembered; the first start follows the
    // desktop's theme mode.
    QObject::connect(&theme, &Theme::darkModeChanged, &theme, [&theme] {
        QSettings("micboard", "micboard-gui").setValue("darkMode", theme.darkMode());
    });

    Backend backend;

    QQmlApplicationEngine engine;
    QObject::connect(&translator, &Translator::changed, &engine, &QQmlEngine::retranslate);
    engine.rootContext()->setContextProperty("translator", &translator);
    engine.rootContext()->setContextProperty("theme", &theme);
    engine.rootContext()->setContextProperty("backend", &backend);
    engine.load(QUrl("qrc:/Main.qml"));
    if (engine.rootObjects().isEmpty())
        return 1;

    // A headless screenshot is handy for the README and for checking a theme
    // change without opening a window on the user's desktop.
    const int screenshotIndex = arguments.indexOf("--screenshot");
    if (screenshotIndex > 0 && screenshotIndex + 1 < arguments.size()) {
        const QString path = arguments.at(screenshotIndex + 1);
        auto *window = qobject_cast<QQuickWindow *>(engine.rootObjects().first());
        if (window) {
            QTimer::singleShot(1200, &app, [&app, window, path] {
                window->grabWindow().save(path);
                app.quit();
            });
        }
    }

    return app.exec();
}
