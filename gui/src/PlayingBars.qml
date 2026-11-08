import QtQuick

// Three bars that bounce while a sound is audible. They appear on playing
// cards and in the "now playing" bar, so it is obvious what is running.
Item {
    id: bars

    property bool running: false
    property color barColor: theme.gold

    implicitWidth: 18
    implicitHeight: 14

    Repeater {
        model: [
            { offset: 0, duration: 420, low: 0.25 },
            { offset: 7, duration: 300, low: 0.55 },
            { offset: 14, duration: 520, low: 0.35 }
        ]

        Rectangle {
            id: bar

            property real level: modelData.low

            x: modelData.offset
            width: 3
            radius: 1.5
            color: bars.barColor
            height: bars.running ? bars.height * level : 4
            y: bars.height - height

            SequentialAnimation on level {
                running: bars.running
                loops: Animation.Infinite
                NumberAnimation {
                    to: modelData.low
                    duration: modelData.duration
                    easing.type: Easing.InOutSine
                }
                NumberAnimation {
                    to: 1.0
                    duration: modelData.duration
                    easing.type: Easing.InOutSine
                }
            }
        }
    }
}
