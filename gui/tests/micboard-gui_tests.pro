QT += core gui qml quick quickcontrols2 testlib dbus

CONFIG += c++17 testcase
TARGET = micboard-gui_tests
TEMPLATE = app

INCLUDEPATH += ../src

HEADERS += \
    ../src/backend.h \
    ../src/theme.h \
    ../src/translator.h

SOURCES += \
    micboard-gui_tests.cpp \
    ../src/backend.cpp \
    ../src/theme.cpp \
    ../src/translator.cpp
