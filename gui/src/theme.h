#pragma once

#include <QObject>
#include <QProcess>
#include <QString>
#include <QTimer>

// The palette is C.C. from Code Geass: warm parchment, mint hair, gold eyes,
// and a deep green-black. The dark variant keeps the same accents on a
// green-black ground, so the window still belongs to the same character.
// The radius follows Hyprland so the app sits comfortably on a themed desktop.
class Theme : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool darkMode READ darkMode WRITE setDarkMode NOTIFY darkModeChanged)
    Q_PROPERTY(QString bg READ bg NOTIFY changed)
    Q_PROPERTY(QString panel READ panel NOTIFY changed)
    Q_PROPERTY(QString panelAlt READ panelAlt NOTIFY changed)
    Q_PROPERTY(QString panelHover READ panelHover NOTIFY changed)
    Q_PROPERTY(QString mintSoft READ mintSoft NOTIFY changed)
    Q_PROPERTY(QString mint READ mint NOTIFY changed)
    Q_PROPERTY(QString mintDeep READ mintDeep NOTIFY changed)
    Q_PROPERTY(QString gold READ gold NOTIFY changed)
    Q_PROPERTY(QString goldSoft READ goldSoft NOTIFY changed)
    Q_PROPERTY(QString ink READ ink NOTIFY changed)
    Q_PROPERTY(QString muted READ muted NOTIFY changed)
    Q_PROPERTY(QString line READ line NOTIFY changed)
    Q_PROPERTY(QString dark READ dark NOTIFY changed)
    Q_PROPERTY(QString danger READ danger NOTIFY changed)
    Q_PROPERTY(QString dangerSoft READ dangerSoft NOTIFY changed)
    Q_PROPERTY(QString onDark READ onDark NOTIFY changed)
    Q_PROPERTY(QString overlay READ overlay NOTIFY changed)
    Q_PROPERTY(QString dropOverlay READ dropOverlay NOTIFY changed)
    Q_PROPERTY(int radius READ radius NOTIFY changed)

public:
    explicit Theme(const QString &hyprctl = {}, QObject *parent = nullptr);
    ~Theme() override;

    bool darkMode() const { return m_dark; }
    void setDarkMode(bool dark);
    Q_INVOKABLE void toggleDarkMode() { setDarkMode(!m_dark); }

    QString bg() const { return m_dark ? "#121A16" : "#F2EDE1"; }
    QString panel() const { return m_dark ? "#1A241F" : "#FBF8F0"; }
    QString panelAlt() const { return m_dark ? "#24312A" : "#E9E1D0"; }
    QString panelHover() const { return m_dark ? "#202C26" : "#F7F2E7"; }
    QString mintSoft() const { return m_dark ? "#24382D" : "#E2F0E6"; }
    QString mint() const { return "#A8D5B8"; }
    QString mintDeep() const { return m_dark ? "#7FBF9A" : "#5E9C7A"; }
    QString gold() const { return m_dark ? "#D8B654" : "#C9A63C"; }
    QString goldSoft() const { return m_dark ? "#3A3320" : "#F0E4BF"; }
    QString ink() const { return m_dark ? "#EFE8D8" : "#24261F"; }
    QString muted() const { return m_dark ? "#9AA79E" : "#7D786A"; }
    QString line() const { return m_dark ? "#2E3B33" : "#DCD3C0"; }
    QString dark() const { return m_dark ? "#0B100D" : "#1E2A24"; }
    QString danger() const { return m_dark ? "#E08063" : "#B25A45"; }
    QString dangerSoft() const { return m_dark ? "#3B2621" : "#F7E3DD"; }
    QString onDark() const { return "#F8F4EA"; }
    QString overlay() const { return m_dark ? "#99000000" : "#66000000"; }
    QString dropOverlay() const { return m_dark ? "#D90B100D" : "#CC1E2A24"; }

    int radius() const { return m_radius; }

    // Omarchy writes the active theme's mode next to its palette; a fresh
    // install has no file, in which case the light parchment palette wins.
    static bool omarchyPrefersDark();

signals:
    void changed();
    void darkModeChanged();

private:
    void refreshRounding();

    QString m_hyprctl;
    bool m_dark = false;
    int m_radius = 0;
    QProcess m_process;
    QTimer m_poll;
    QTimer m_timeout;
};
