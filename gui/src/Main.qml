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
    property string query: ""
    property bool searchFocused: false

    readonly property var visibleSounds: {
        const needle = win.query.toLowerCase();
        if (needle.length === 0)
            return backend.sounds;
        return backend.sounds.filter(function(sound) {
            const haystack = (sound.id + " " + sound.file + " " + (sound.hotkey || "")).toLowerCase();
            return haystack.indexOf(needle) >= 0;
        });
    }

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
        sequence: "Ctrl+F"
        context: Qt.ApplicationShortcut
        onActivated: {
            searchField.forceActiveFocus();
            searchField.selectAll();
        }
    }
    Shortcut {
        sequence: "?"
        context: Qt.ApplicationShortcut
        enabled: !win.searchFocused
        onActivated: win.helpVisible = !win.helpVisible
    }
    Shortcut {
        sequence: "Space"
        context: Qt.ApplicationShortcut
        enabled: !win.searchFocused
        onActivated: win.playSelected()
    }
    Shortcut {
        sequence: "S"
        context: Qt.ApplicationShortcut
        enabled: !win.searchFocused
        onActivated: backend.stopAll()
    }
    Shortcut {
        sequence: "R"
        context: Qt.ApplicationShortcut
        enabled: !win.searchFocused
        onActivated: backend.reloadConfig()
    }
    Shortcut {
        sequence: "A"
        context: Qt.ApplicationShortcut
        enabled: !win.searchFocused
        onActivated: backend.chooseFile()
    }
    Shortcut {
        sequence: "T"
        context: Qt.ApplicationShortcut
        enabled: !win.searchFocused
        onActivated: theme.toggleDarkMode()
    }
    Shortcut {
        sequence: "Q"
        context: Qt.ApplicationShortcut
        enabled: !win.searchFocused
        onActivated: Qt.quit()
    }
    Shortcut {
        sequence: "Escape"
        context: Qt.ApplicationShortcut
        onActivated: {
            if (win.searchFocused) {
                searchField.text = "";
                win.query = "";
            }
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
                text: theme.darkMode ? "☀" : "☾"
                onClicked: theme.toggleDarkMode()
                ToolTip.visible: hovered
                ToolTip.text: theme.darkMode ? qsTr("Light theme") : qsTr("Dark theme")
                ToolTip.delay: 400
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
                        text: win.query.length > 0
                              ? qsTr("%n found", "", win.visibleSounds.length)
                              : qsTr("%n sounds", "", backend.soundCount)
                        color: theme.muted
                        font.pixelSize: 10
                    }
                }

                Item { Layout.fillWidth: true }

                Rectangle {
                    Layout.preferredWidth: 200
                    Layout.minimumWidth: 120
                    Layout.preferredHeight: 34
                    radius: 10
                    color: theme.panel
                    border.width: 1
                    border.color: searchField.activeFocus ? theme.mintDeep : theme.line

                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: 12
                        anchors.verticalCenter: parent.verticalCenter
                        visible: searchField.text.length === 0
                        text: qsTr("Search sounds")
                        color: theme.muted
                        font.pixelSize: 12
                    }

                    TextInput {
                        id: searchField
                        anchors.fill: parent
                        anchors.leftMargin: 12
                        anchors.rightMargin: 26
                        verticalAlignment: TextInput.AlignVCenter
                        color: theme.ink
                        selectionColor: theme.mint
                        selectedTextColor: theme.dark
                        font.pixelSize: 12
                        clip: true
                        onTextChanged: win.query = text.trim()
                        onActiveFocusChanged: win.searchFocused = activeFocus
                        Keys.onReturnPressed: {
                            if (win.visibleSounds.length > 0) {
                                win.selectedId = win.visibleSounds[0].id;
                                backend.play(win.visibleSounds[0].id);
                            }
                        }
                    }

                    Text {
                        anchors.right: parent.right
                        anchors.rightMargin: 9
                        anchors.verticalCenter: parent.verticalCenter
                        visible: searchField.text.length > 0
                        text: "×"
                        color: clearHover.hovered ? theme.ink : theme.muted
                        font.pixelSize: 16

                        HoverHandler { id: clearHover }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                searchField.text = "";
                                win.query = "";
                                searchField.forceActiveFocus();
                            }
                        }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 54
                radius: 14
                color: backend.playing > 0 ? theme.mintSoft : theme.panel
                border.width: 1
                border.color: backend.playing > 0 ? theme.mintDeep : theme.line

                Behavior on color { ColorAnimation { duration: 120 } }

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 16
                    anchors.rightMargin: 10
                    spacing: 12

                    PlayingBars {
                        running: backend.playing > 0
                        barColor: theme.mintDeep
                        Layout.preferredWidth: 18
                        Layout.preferredHeight: 14
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 1

                        Text {
                            Layout.fillWidth: true
                            text: backend.playing > 0
                                  ? qsTr("Now playing")
                                  : backend.running
                                    ? qsTr("Nothing is playing")
                                    : qsTr("Daemon offline")
                            color: backend.playing > 0 ? theme.ink : theme.muted
                            font.pixelSize: 12
                            font.weight: backend.playing > 0 ? Font.DemiBold : Font.Normal
                            elide: Text.ElideRight
                        }

                        Text {
                            Layout.fillWidth: true
                            visible: backend.playing > 0
                            text: backend.playingIds.join("  ·  ")
                            color: theme.mintDeep
                            font.pixelSize: 11
                            elide: Text.ElideRight
                        }
                    }

                    SoftButton {
                        text: qsTr("Stop all")
                        active: backend.playing > 0
                        onClicked: backend.stopAll()
                    }
                }
            }

            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                Flickable {
                    id: scroll
                    anchors.fill: parent
                    clip: true
                    visible: win.visibleSounds.length > 0
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
                            model: win.visibleSounds

                            SoundCard {
                                width: win.cardWidth(flow.width)
                                sound: modelData
                                selected: win.selectedId === modelData.id
                                playing: backend.playingIds.indexOf(modelData.id) >= 0

                                onPlayRequested: {
                                    win.selectedId = modelData.id;
                                    backend.play(modelData.id);
                                }

                                onStopRequested: backend.stopSound(modelData.id)

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
                    visible: win.visibleSounds.length === 0
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
                                text: backend.soundCount === 0 ? "♪" : "⌕"
                                color: theme.mintDeep
                                font.family: "Noto Serif"
                                font.pixelSize: 32
                            }
                        }

                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            text: backend.soundCount === 0
                                  ? qsTr("No sounds yet")
                                  : qsTr("Nothing found")
                            color: theme.ink
                            font.family: "Noto Serif"
                            font.pixelSize: 23
                        }

                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            Layout.fillWidth: true
                            text: backend.soundCount === 0
                                  ? qsTr("Drop audio or video files here, or press Add sound.")
                                  : qsTr("Try a different search.")
                            color: theme.muted
                            font.pixelSize: 12
                            horizontalAlignment: Text.AlignHCenter
                            wrapMode: Text.WordWrap
                        }

                        SoftButton {
                            Layout.alignment: Qt.AlignHCenter
                            visible: backend.soundCount === 0
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
                        text: "Ctrl+F  ·  Space  ·  S  ·  T"
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
            color: theme.dropOverlay

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
                    color: theme.onDark
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
            color: theme.onDark
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
        color: theme.overlay

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
        color: theme.overlay

        MouseArea {
            anchors.fill: parent
            onClicked: win.helpVisible = false
        }

        Rectangle {
            anchors.centerIn: parent
            width: 460
            height: 500
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
                        { key: "Ctrl+F", text: qsTr("Search sounds") },
                        { key: "Space", text: qsTr("Play selected") },
                        { key: "S", text: qsTr("Stop everything") },
                        { key: "R", text: qsTr("Reload config") },
                        { key: "A", text: qsTr("Add a sound") },
                        { key: "T", text: qsTr("Toggle theme") },
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
