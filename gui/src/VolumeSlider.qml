import QtQuick

Item {
    id: slider

    property real value: 0
    property real from: 0
    property real to: 1.5

    signal moved(real value)

    implicitHeight: 26

    function ratio() {
        return Math.max(0, Math.min(1, (value - from) / (to - from)));
    }

    Rectangle {
        id: track
        anchors.verticalCenter: parent.verticalCenter
        width: parent.width
        height: 6
        radius: 3
        color: theme.panelAlt
    }

    Rectangle {
        anchors.verticalCenter: parent.verticalCenter
        width: track.width * slider.ratio()
        height: 6
        radius: 3
        color: theme.mintDeep
    }

    Rectangle {
        id: handle
        anchors.verticalCenter: parent.verticalCenter
        x: Math.max(0, Math.min(track.width - width, track.width * slider.ratio() - width / 2))
        width: 20
        height: 20
        radius: 10
        color: theme.panel
        border.width: 2
        border.color: theme.mintDeep
    }

    MouseArea {
        anchors.fill: parent
        anchors.topMargin: -10
        anchors.bottomMargin: -10
        cursorShape: Qt.PointingHandCursor

        function update(x) {
            const ratio = Math.max(0, Math.min(1, x / track.width));
            slider.moved(slider.from + ratio * (slider.to - slider.from));
        }

        onPressed: update(mouse.x)
        onPositionChanged: if (pressed) update(mouse.x)
    }
}
