import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: win

    width: 1080
    height: 720
    minimumWidth: 860
    minimumHeight: 600
    visible: true
    title: qsTr("MicBoard")
    color: theme.bg
    font.family: "Noto Sans"
    font.pixelSize: 13

    property string selectedId: ""
    property bool helpVisible: false
    property bool removeDialogVisible: false
    property string pendingRemoveId: ""
    property string pendingRemoveName: ""

    function cardWidth(available) {
        const columns = Math.max(1, Math.floor((available + 14) / 300));
        // Leave a pixel of slack: Flow wraps when a card ends exactly on the
        // right edge, which would silently drop the grid to one column.
        return Math.floor((available - 14 * (columns - 1)) / columns) - 1;
    }

    function playSelected() {
        if (win.selectedId.length > 0)
            backend.play(win.selectedId);
    }

    Shortcut {
        sequence: "?"
        context: Qt.ApplicationShortcut
        onActivated: win.helpVisible = !win.helpVisible
    }
    Shortcut {
        sequence: "Space"
        context: Qt.ApplicationShortcut
        onActivated: win.playSelected()
    }
    Shortcut {
        sequence: "S"
        context: Qt.ApplicationShortcut
        onActivated: backend.stopAll()
    }
    Shortcut {
        sequence: "R"
        context: Qt.ApplicationShortcut
        onActivated: backend.reloadConfig()
    }
    Shortcut {
        sequence: "A"
        context: Qt.ApplicationShortcut
        onActivated: backend.chooseFile()
    }
    Shortcut {
        sequence: "Q"
        context: Qt.ApplicationShortcut
        onActivated: Qt.quit()
    }
    Shortcut {
        sequence: "Escape"
        context: Qt.ApplicationShortcut
        onActivated: {
            win.helpVisible = false;
            win.removeDialogVisible = false;
        }
    }

    Rectangle {
        id: header
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        height: 88
        color: theme.bg

        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: 1
            color: theme.line
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 28
            anchors.rightMargin: 28
            spacing: 14

            Rectangle {
                Layout.preferredWidth: 48
                Layout.preferredHeight: 48
                radius: 16
                color: theme.dark

                Rectangle {
                    anchors.centerIn: parent
                    width: 30
                    height: 30
                    radius: 15
                    color: "transparent"
                    border.width: 2
                    border.color: theme.mint
                }

                Rectangle {
                    anchors.centerIn: parent
                    width: 8
                    height: 8
                    radius: 4
                    color: theme.gold
                }
            }

            ColumnLayout {
                spacing: 1

                Text {
                    text: "MicBoard"
                    color: theme.ink
                    font.family: "Noto Serif"
                    font.pixelSize: 25
                    font.weight: Font.DemiBold
                }

                Text {
                    text: qsTr("Soundboard for your microphone")
                    color: theme.muted
                    font.pixelSize: 12
                }
            }

            Item { Layout.fillWidth: true }

            Rectangle {
                Layout.preferredHeight: 34
                Layout.preferredWidth: statusRow.implicitWidth + 24
                radius: 17
                color: backend.running ? theme.mintSoft : theme.panelAlt
                border.width: 1
                border.color: backend.running ? theme.mintDeep : theme.line

                RowLayout {
                    id: statusRow
                    anchors.centerIn: parent
                    spacing: 8

                    Rectangle {
                        width: 8
                        height: 8
                        radius: 4
                        color: backend.starting
                               ? theme.gold
                               : backend.running
                                 ? theme.mintDeep
                                 : theme.muted
                    }

                    Text {
                        text: backend.starting
                              ? qsTr("Starting…")
                              : backend.running
                                ? qsTr("Daemon running")
                                : qsTr("Daemon offline")
                        color: theme.ink
                        font.pixelSize: 11
                    }
                }
            }

            SoftButton {
                text: backend.running ? qsTr("Stop daemon") : qsTr("Start daemon")
                active: !backend.starting
                onClicked: backend.running ? backend.stopDaemon() : backend.startDaemon()
            }

            SoftButton {
                text: "+  " + qsTr("Add sound")
                primary: true
                onClicked: backend.chooseFile()
            }
        }
    }

    RowLayout {
        anchors.top: header.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: 24
        spacing: 20

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 14

            RowLayout {
                spacing: 10

                Text {
                    text: qsTr("Sounds")
                    color: theme.ink
                    font.family: "Noto Serif"
                    font.pixelSize: 23
                    font.weight: Font.DemiBold
                }

                Rectangle {
                    Layout.preferredHeight: 24
                    Layout.preferredWidth: countText.implicitWidth + 18
                    radius: 12
                    color: theme.panel
                    border.width: 1
                    border.color: theme.line

                    Text {
                        id: countText
                        anchors.centerIn: parent
                        text: qsTr("%n sounds", "", backend.soundCount)
                        color: theme.muted
                        font.pixelSize: 10
                    }
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: qsTr("Click a card to play")
                    color: theme.muted
                    font.pixelSize: 11
                }
            }

            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                Flickable {
                    id: scroll
                    anchors.fill: parent
                    clip: true
                    visible: backend.soundCount > 0
                    contentHeight: flow.height
                    boundsBehavior: Flickable.StopAtBounds

                    ScrollBar.vertical: ScrollBar {
                        policy: ScrollBar.AsNeeded
                        width: 6
                    }

                    Flow {
                        id: flow
                        width: scroll.width - 12
                        spacing: 14

                        Repeater {
                            model: backend.sounds

                            SoundCard {
                                width: win.cardWidth(flow.width)
                                sound: modelData
                                selected: win.selectedId === modelData.id
                                playing: backend.playing > 0
                                         && backend.lastPlayedId === modelData.id

                                onPlayRequested: {
                                    win.selectedId = modelData.id;
                                    backend.play(modelData.id);
                                }

                                onRemoveRequested: {
                                    win.pendingRemoveId = modelData.id;
                                    win.pendingRemoveName = modelData.id;
                                    win.removeDialogVisible = true;
                                }
                            }
                        }
                    }
                }

                Rectangle {
                    anchors.fill: parent
                    visible: backend.soundCount === 0
                    radius: 20
                    color: theme.panel
                    border.width: 1
                    border.color: theme.line

                    ColumnLayout {
                        anchors.centerIn: parent
                        width: Math.min(parent.width - 80, 420)
                        spacing: 12

                        Rectangle {
                            Layout.alignment: Qt.AlignHCenter
                            width: 68
                            height: 68
                            radius: 34
                            color: theme.mintSoft

                            Text {
                                anchors.centerIn: parent
                                text: "♪"
                                color: theme.mintDeep
                                font.family: "Noto Serif"
                                font.pixelSize: 32
                            }
                        }

                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            text: qsTr("No sounds yet")
                            color: theme.ink
                            font.family: "Noto Serif"
                            font.pixelSize: 23
                        }

                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            Layout.fillWidth: true
                            text: qsTr("Drop audio or video files here, or press Add sound.")
                            color: theme.muted
                            font.pixelSize: 12
                            horizontalAlignment: Text.AlignHCenter
                            wrapMode: Text.WordWrap
                        }

                        SoftButton {
                            Layout.alignment: Qt.AlignHCenter
                            text: "+  " + qsTr("Add sound")
                            primary: true
                            onClicked: backend.chooseFile()
                        }
                    }
                }
            }
        }

        ColumnLayout {
            Layout.preferredWidth: 320
            Layout.minimumWidth: 320
            Layout.maximumWidth: 320
            Layout.fillHeight: true
            spacing: 14

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 290
                radius: 20
                color: theme.panel
                border.width: 1
                border.color: theme.line

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 20
                    spacing: 12

                    Text {
                        text: qsTr("Output")
                        color: theme.ink
                        font.family: "Noto Serif"
                        font.pixelSize: 19
                        font.weight: Font.DemiBold
                    }

                    RowLayout {
                        Layout.fillWidth: true

                        Text {
                            text: qsTr("Master volume")
                            color: theme.muted
                            font.pixelSize: 11
                        }

                        Item { Layout.fillWidth: true }

                        Text {
                            text: Math.round(backend.masterVolume * 100) + "%"
                            color: theme.ink
                            font.pixelSize: 12
                            font.weight: Font.DemiBold
                        }
                    }

                    VolumeSlider {
                        Layout.fillWidth: true
                        value: backend.masterVolume
                        onMoved: function(value) {
                            backend.setMasterVolume(value);
                        }
                    }

                    Item { Layout.preferredHeight: 2 }

                    SoftButton {
                        Layout.fillWidth: true
                        text: qsTr("Stop all")
                        dark: true
                        onClicked: backend.stopAll()
                    }

                    SoftButton {
                        Layout.fillWidth: true
                        text: qsTr("Reload config")
                        onClicked: backend.reloadConfig()
                    }

                    SoftButton {
                        Layout.fillWidth: true
                        text: qsTr("Open config")
                        flat: true
                        onClicked: backend.openConfig()
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 190
                radius: 20
                color: theme.panel
                border.width: 1
                border.color: theme.line

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 20
                    spacing: 8

                    Text {
                        text: qsTr("Virtual microphone")
                        color: theme.ink
                        font.family: "Noto Serif"
                        font.pixelSize: 18
                        font.weight: Font.DemiBold
                    }

                    Text {
                        Layout.fillWidth: true
                        text: backend.source.length > 0 ? backend.source : "MicBoard_Mic"
                        color: theme.mintDeep
                        font.pixelSize: 13
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                    }

                    Text {
                        Layout.fillWidth: true
                        text: qsTr("Choose %1 as the microphone in Discord, OBS or your browser.")
                                  .arg(backend.source.length > 0 ? backend.source : "MicBoard_Mic")
                        color: theme.muted
                        font.pixelSize: 11
                        wrapMode: Text.WordWrap
                    }

                    Item { Layout.fillHeight: true }

                    RowLayout {
                        spacing: 8

                        Rectangle {
                            height: 22
                            width: modeText.implicitWidth + 16
                            radius: 11
                            color: theme.goldSoft
                            border.width: 1
                            border.color: theme.gold

                            Text {
                                id: modeText
                                anchors.centerIn: parent
                                text: backend.mode === "exclusive"
                                      ? qsTr("Exclusive")
                                      : qsTr("Overlap")
                                color: theme.ink
                                font.pixelSize: 10
                            }
                        }

                        Rectangle {
                            height: 22
                            width: hotkeyText.implicitWidth + 16
                            radius: 11
                            color: theme.mintSoft
                            border.width: 1
                            border.color: theme.mintDeep

                            Text {
                                id: hotkeyText
                                anchors.centerIn: parent
                                text: qsTr("Hotkeys: %1").arg(backend.hotkeys)
                                color: theme.ink
                                font.pixelSize: 10
                            }
                        }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 58
                radius: 16
                color: theme.panelAlt
                border.width: 1
                border.color: theme.line

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 16
                    anchors.rightMargin: 16
                    spacing: 10

                    Rectangle {
                        width: 24
                        height: 24
                        radius: 7
                        color: theme.panel
                        border.width: 1
                        border.color: theme.line

                        Text {
                            anchors.centerIn: parent
                            text: "?"
                            color: theme.ink
                            font.pixelSize: 12
                        }
                    }

                    Text {
                        text: qsTr("Keyboard shortcuts")
                        color: theme.muted
                        font.pixelSize: 11
                    }

                    Item { Layout.fillWidth: true }

                    Text {
                        text: "Space  ·  S  ·  R"
                        color: theme.muted
                        font.pixelSize: 10
                    }
                }
            }
        }
    }

    DropArea {
        anchors.fill: parent
        z: 45
        onDropped: function(drop) {
            backend.addDropped(drop.urls);
        }

        Rectangle {
            anchors.fill: parent
            visible: parent.containsDrag
            color: "#CC1E2A24"

            ColumnLayout {
                anchors.centerIn: parent
                spacing: 10

                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "↓"
                    color: theme.mint
                    font.pixelSize: 42
                }

                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: qsTr("Drop files to add them")
                    color: "#F8F4EA"
                    font.family: "Noto Serif"
                    font.pixelSize: 24
                }
            }
        }
    }

    Rectangle {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 22
        z: 50
        visible: backend.message.length > 0
        height: 42
        width: Math.min(win.width - 80, toastText.implicitWidth + 40)
        radius: 21
        color: backend.messageIsError ? theme.danger : theme.dark

        Text {
            id: toastText
            anchors.centerIn: parent
            width: parent.width - 32
            text: backend.message
            color: "#F8F4EA"
            font.pixelSize: 12
            horizontalAlignment: Text.AlignHCenter
            elide: Text.ElideRight
        }

        Timer {
            interval: 6000
            running: backend.message.length > 0
            repeat: false
            onTriggered: backend.clearMessage()
        }
    }

    Rectangle {
        anchors.fill: parent
        z: 55
        visible: win.removeDialogVisible
        color: "#66000000"

        MouseArea {
            anchors.fill: parent
            onClicked: win.removeDialogVisible = false
        }

        Rectangle {
            anchors.centerIn: parent
            width: 430
            height: 230
            radius: 22
            color: theme.panel
            border.width: 1
            border.color: theme.line

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 24
                spacing: 12

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Remove “%1”?").arg(win.pendingRemoveName)
                    color: theme.ink
                    font.family: "Noto Serif"
                    font.pixelSize: 21
                    wrapMode: Text.WordWrap
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("This only removes it from MicBoard. The file stays where it is.")
                    color: theme.muted
                    font.pixelSize: 12
                    wrapMode: Text.WordWrap
                }

                Item { Layout.fillHeight: true }

                RowLayout {
                    Layout.alignment: Qt.AlignRight
                    spacing: 10

                    SoftButton {
                        text: qsTr("Cancel")
                        onClicked: win.removeDialogVisible = false
                    }

                    SoftButton {
                        text: qsTr("Remove")
                        danger: true
                        onClicked: {
                            backend.removeSound(win.pendingRemoveId);
                            win.removeDialogVisible = false;
                        }
                    }
                }
            }
        }
    }

    Rectangle {
        anchors.fill: parent
        z: 60
        visible: win.helpVisible
        color: "#66000000"

        MouseArea {
            anchors.fill: parent
            onClicked: win.helpVisible = false
        }

        Rectangle {
            anchors.centerIn: parent
            width: 460
            height: 410
            radius: 22
            color: theme.panel
            border.width: 1
            border.color: theme.line

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 26
                spacing: 12

                Text {
                    text: qsTr("Keyboard shortcuts")
                    color: theme.ink
                    font.family: "Noto Serif"
                    font.pixelSize: 22
                }

                Repeater {
                    model: [
                        { key: "Space", text: qsTr("Play selected") },
                        { key: "S", text: qsTr("Stop everything") },
                        { key: "R", text: qsTr("Reload config") },
                        { key: "A", text: qsTr("Add a sound") },
                        { key: "Q", text: qsTr("Quit") },
                        { key: "?", text: qsTr("Show this help") }
                    ]

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 14

                        Rectangle {
                            width: 64
                            height: 28
                            radius: 8
                            color: theme.panelAlt
                            border.width: 1
                            border.color: theme.line

                            Text {
                                anchors.centerIn: parent
                                text: modelData.key
                                color: theme.ink
                                font.pixelSize: 11
                            }
                        }

                        Text {
                            text: modelData.text
                            color: theme.muted
                            font.pixelSize: 12
                        }

                        Item { Layout.fillWidth: true }
                    }
                }

                Item { Layout.fillHeight: true }

                SoftButton {
                    Layout.alignment: Qt.AlignRight
                    text: qsTr("Close")
                    onClicked: win.helpVisible = false
                }
            }
        }
    }
}
