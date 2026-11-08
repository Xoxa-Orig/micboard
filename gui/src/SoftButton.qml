import QtQuick

Rectangle {
    id: button

    property string text
    property bool primary: false
    property bool dark: false
    property bool danger: false
    property bool flat: false
    property bool active: true
    property alias hovered: hover.hovered

    signal clicked()

    implicitWidth: label.implicitWidth + 30
    implicitHeight: 38
    radius: 11
    opacity: active ? 1 : 0.45

    color: !active
           ? theme.panelAlt
           : primary
             ? theme.mint
             : dark
               ? theme.dark
               : danger
                 ? theme.danger
                 : flat
                   ? "transparent"
                   : theme.panel

    border.width: flat ? 0 : 1
    border.color: primary
                  ? theme.mintDeep
                  : dark
                    ? (theme.darkMode ? theme.mintDeep : theme.dark)
                    : danger
                      ? theme.danger
                      : theme.line

    Text {
        id: label
        anchors.centerIn: parent
        text: button.text
        color: button.primary
               ? theme.dark
               : button.dark || button.danger
                 ? theme.onDark
                 : theme.ink
        font.family: "Noto Sans"
        font.pixelSize: 12
        font.weight: Font.Medium
    }

    HoverHandler {
        id: hover
        enabled: button.active
    }

    Rectangle {
        anchors.fill: parent
        radius: button.radius
        color: theme.ink
        opacity: hover.hovered && button.active ? 0.05 : 0
        Behavior on opacity { NumberAnimation { duration: 120 } }
    }

    MouseArea {
        anchors.fill: parent
        enabled: button.active
        cursorShape: Qt.PointingHandCursor
        onClicked: button.clicked()
    }
}
