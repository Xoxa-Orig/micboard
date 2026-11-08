import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: card

    property var sound: ({})
    property bool selected: false
    property bool playing: false

    signal playRequested()
    signal stopRequested()
    signal removeRequested()

    implicitHeight: 128
    radius: 16
    color: playing ? theme.goldSoft
                  : selected ? theme.mintSoft
                             : cardHover.hovered ? theme.panelHover : theme.panel
    border.width: playing || selected ? 2 : 1
    border.color: playing ? theme.gold : selected ? theme.mintDeep : theme.line

    Behavior on color { ColorAnimation { duration: 120 } }

    HoverHandler {
        id: cardHover
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: card.playRequested()
    }

    RowLayout {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 14

        Rectangle {
            Layout.alignment: Qt.AlignTop
            width: 48
            height: 48
            radius: 24
            color: card.playing ? theme.gold : theme.mint
            border.width: card.playing ? 0 : 1
            border.color: theme.mintDeep

            Behavior on color { ColorAnimation { duration: 150 } }

            Text {
                anchors.centerIn: parent
                anchors.horizontalCenterOffset: card.playing ? 0 : 2
                text: card.playing ? "■" : "▶"
                color: theme.dark
                font.family: "Noto Sans"
                font.pixelSize: 16
            }

            HoverHandler { id: playHover }

            ToolTip.visible: playHover.hovered
            ToolTip.text: card.playing ? qsTr("Stop this sound") : qsTr("Play")
            ToolTip.delay: 400

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: card.playing ? card.stopRequested() : card.playRequested()
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignVCenter
            spacing: 5

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                PlayingBars {
                    visible: card.playing
                    running: card.playing
                    barColor: theme.gold
                    Layout.preferredWidth: 18
                    Layout.preferredHeight: 14
                }

                Text {
                    Layout.fillWidth: true
                    text: card.sound.id || ""
                    color: theme.ink
                    font.family: "Noto Sans"
                    font.pixelSize: 15
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }

                Rectangle {
                    visible: cardHover.hovered || card.selected
                    width: 26
                    height: 26
                    radius: 13
                    color: removeHover.hovered ? theme.panelAlt : "transparent"

                    HoverHandler {
                        id: removeHover
                    }

                    Text {
                        anchors.centerIn: parent
                        text: "×"
                        color: theme.muted
                        font.family: "Noto Sans"
                        font.pixelSize: 18
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: card.removeRequested()
                    }
                }
            }

            Text {
                Layout.fillWidth: true
                text: (card.sound.file || "").replace(/^.*[\\/]/, "")
                color: theme.muted
                font.family: "Noto Sans"
                font.pixelSize: 11
                elide: Text.ElideMiddle
            }

            RowLayout {
                spacing: 8

                Rectangle {
                    visible: !!card.sound.hotkey
                    height: 21
                    width: hotkey.implicitWidth + 16
                    radius: 10
                    color: theme.goldSoft
                    border.width: 1
                    border.color: theme.gold

                    Text {
                        id: hotkey
                        anchors.centerIn: parent
                        text: card.sound.hotkey || ""
                        color: theme.ink
                        font.family: "Noto Sans"
                        font.pixelSize: 10
                    }
                }

                Rectangle {
                    visible: card.sound.exists === false
                    height: 21
                    width: missing.implicitWidth + 16
                    radius: 10
                    color: theme.dangerSoft
                    border.width: 1
                    border.color: theme.danger

                    Text {
                        id: missing
                        anchors.centerIn: parent
                        text: qsTr("missing")
                        color: theme.danger
                        font.family: "Noto Sans"
                        font.pixelSize: 10
                    }
                }

                Rectangle {
                    visible: card.playing
                    height: 21
                    width: playingLabel.implicitWidth + 16
                    radius: 10
                    color: theme.mintSoft
                    border.width: 1
                    border.color: theme.mintDeep

                    Text {
                        id: playingLabel
                        anchors.centerIn: parent
                        text: qsTr("Playing")
                        color: theme.mintDeep
                        font.family: "Noto Sans"
                        font.pixelSize: 10
                        font.weight: Font.DemiBold
                    }
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: Math.round((card.sound.volume || 1) * 100) + "%"
                    color: theme.muted
                    font.family: "Noto Sans"
                    font.pixelSize: 10
                }
            }
        }
    }
}
