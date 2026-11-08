#pragma once

#include <QObject>
#include <QProcess>
#include <QString>
#include <QTimer>

// The palette is C.C. from Code Geass: warm parchment, mint hair, gold eyes,
// and a deep green-black. The radius follows Hyprland so the app still sits
// comfortably on a themed desktop.
class Theme : public QObject {
    Q_OBJECT
    Q_PROPERTY(QString bg READ bg CONSTANT)
    Q_PROPERTY(QString panel READ panel CONSTANT)
    Q_PROPERTY(QString panelAlt READ panelAlt CONSTANT)
    Q_PROPERTY(QString panelHover READ panelHover CONSTANT)
    Q_PROPERTY(QString mintSoft READ mintSoft CONSTANT)
    Q_PROPERTY(QString mint READ mint CONSTANT)
    Q_PROPERTY(QString mintDeep READ mintDeep CONSTANT)
    Q_PROPERTY(QString gold READ gold CONSTANT)
    Q_PROPERTY(QString goldSoft READ goldSoft CONSTANT)
    Q_PROPERTY(QString ink READ ink CONSTANT)
    Q_PROPERTY(QString muted READ muted CONSTANT)
    Q_PROPERTY(QString line READ line CONSTANT)
    Q_PROPERTY(QString dark READ dark CONSTANT)
    Q_PROPERTY(QString danger READ danger CONSTANT)
    Q_PROPERTY(int radius READ radius NOTIFY changed)

public:
    explicit Theme(const QString &hyprctl = {}, QObject *parent = nullptr);
    ~Theme() override;

    QString bg() const { return "#F2EDE1"; }
    QString panel() const { return "#FBF8F0"; }
    QString panelAlt() const { return "#E9E1D0"; }
    QString panelHover() const { return "#F7F2E7"; }
    QString mintSoft() const { return "#E2F0E6"; }
    QString mint() const { return "#A8D5B8"; }
    QString mintDeep() const { return "#5E9C7A"; }
    QString gold() const { return "#C9A63C"; }
    QString goldSoft() const { return "#F0E4BF"; }
    QString ink() const { return "#24261F"; }
    QString muted() const { return "#7D786A"; }
    QString line() const { return "#DCD3C0"; }
    QString dark() const { return "#1E2A24"; }
    QString danger() const { return "#B25A45"; }

    int radius() const { return m_radius; }

signals:
    void changed();

private:
    void refreshRounding();

    QString m_hyprctl;
    int m_radius = 0;
    QProcess m_process;
    QTimer m_poll;
    QTimer m_timeout;
};
