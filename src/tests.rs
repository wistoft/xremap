use evdev::Key;
use indoc::indoc;
use nix::sys::timerfd::{ClockId, TimerFd, TimerFlags};
use std::path::Path;
use std::time::Duration;

use crate::client::{Client, WMClient};
use crate::device::InputDeviceInfo;
use crate::{
    action::Action,
    config::{keymap::build_keymap_table, Config},
    event::{Event, KeyEvent, KeyValue, RelativeEvent},
    event_handler::EventHandler,
};

struct StaticClient {
    current_application: Option<String>,
}

impl Client for StaticClient {
    fn supported(&mut self) -> bool {
        true
    }
    fn current_window(&mut self) -> Option<String> {
        None
    }

    fn current_application(&mut self) -> Option<String> {
        self.current_application.clone()
    }
}

fn get_input_device_info<'a>() -> InputDeviceInfo<'a> {
    InputDeviceInfo {
        name: "Some Device",
        path: &Path::new("/dev/input/event0"),
    }
}

#[test]
fn test_basic_modmap() {
    assert_actions(
        indoc! {"
        modmap:
          - remap:
              a: b
        "},
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_A, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_A, KeyValue::Release)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_B, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_B, KeyValue::Release)),
        ],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_B, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_B, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_B, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_B, KeyValue::Release)),
        ],
    )
}

/* Table to see which scancodes/custom key events correspond to which relative events
    Original RELATIVE event | scancode | Custom keyname if                              | Info
                            |          | positive value (+)     | negative value (-)    |
    REL_X                   |    0     | XRIGHTCURSOR       | XLEFTCURSOR       | Cursor right and left
    REL_Y                   |    1     | XDOWNCURSOR        | XUPCURSOR         | Cursor down and up
    REL_Z                   |    2     | XREL_Z_AXIS_1      | XREL_Z_AXIS_2     | Cursor... forward and backwards?
    REL_RX                  |    3     | XREL_RX_AXIS_1     | XREL_RX_AXIS_2    | Horizontally rotative cursor movement?
    REL_RY                  |    4     | XREL_RY_AXIS_1     | XREL_RY_AXIS_2    | Vertical rotative cursor movement?
    REL_RZ                  |    5     | XREL_RZ_AXIS_1     | XREL_RZ_AXIS_2    | "Whatever the third dimensional axis is called" rotative cursor movement?
    REL_HWHEEL              |    6     | XRIGHTSCROLL       | XLEFTSCROLL       | Rightscroll and leftscroll
    REL_DIAL                |    7     | XREL_DIAL_1        | XREL_DIAL_2       | ???
    REL_WHEEL               |    8     | XUPSCROLL          | XDOWNSCROLL       | Upscroll and downscroll
    REL_MISC                |    9     | XREL_MISC_1        | XREL_MISC_2       | Something?
    REL_RESERVED            |    10    | XREL_RESERVED_1    | XREL_RESERVED_2   | Something?
    REL_WHEEL_HI_RES        |    11    | XHIRES_UPSCROLL    | XHIRES_DOWNSCROLL | High resolution downscroll and upscroll, sent just after their non-high resolution version
    REL_HWHEEL_HI_RES       |    12    | XHIRES_RIGHTSCROLL | XHIRES_LEFTSCROLL | High resolution rightcroll and leftscroll, sent just after their non-high resolution version
*/

const _POSITIVE: i32 = 1;
const _NEGATIVE: i32 = -1;

const _REL_X: u16 = 0;
const _REL_Y: u16 = 1;
const _REL_Z: u16 = 2;
const _REL_RX: u16 = 3;
const _REL_RY: u16 = 4;
const _REL_RZ: u16 = 5;
const _REL_HWHEEL: u16 = 6;
const _REL_DIAL: u16 = 7;
const _REL_WHEEL: u16 = 8;
const _REL_MISC: u16 = 9;
const _REL_RESERVED: u16 = 10;
const _REL_WHEEL_HI_RES: u16 = 11;
const _REL_HWHEEL_HI_RES: u16 = 12;

#[test]
fn test_relative_events() {
    assert_actions(
        indoc! {"
        modmap:
          - remap:
              XRIGHTCURSOR: b
        "},
        vec![Event::RelativeEvent(
            get_input_device_info(),
            RelativeEvent::new_with(_REL_X, _POSITIVE),
        )],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_B, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_B, KeyValue::Release)),
        ],
    )
}

#[test]
fn verify_disguised_relative_events() {
    use crate::event_handler::DISGUISED_EVENT_OFFSETTER;
    // Verifies that the event offsetter used to "disguise" relative events into key event
    // is a bigger number than the biggest one a scancode had at the time of writing this (26 december 2022)
    assert!(0x2e7 < DISGUISED_EVENT_OFFSETTER);
    // and that it's not big enough that one of the "disguised" events's scancode would overflow.
    // (the largest of those events is equal to DISGUISED_EVENT_OFFSETTER + 26)
    assert!(DISGUISED_EVENT_OFFSETTER <= u16::MAX - 26)
}

#[test]
fn test_mouse_movement_event_accumulation() {
    // Tests that mouse movement events correctly get collected to be sent as one MouseMovementEventCollection,
    // which is necessary to avoid separating mouse movement events with synchronization events,
    // because such a separation would cause a bug with cursor movement.

    // Please refer to test_cursor_behavior_1 and test_cursor_behavior_2 for more information on said bug.
    assert_actions(
        indoc! {""},
        vec![
            Event::RelativeEvent(get_input_device_info(), RelativeEvent::new_with(_REL_X, _POSITIVE)),
            Event::RelativeEvent(get_input_device_info(), RelativeEvent::new_with(_REL_Y, _POSITIVE)),
        ],
        vec![Action::MouseMovementEventCollection(vec![
            RelativeEvent::new_with(_REL_X, _POSITIVE),
            RelativeEvent::new_with(_REL_Y, _POSITIVE),
        ])],
    )
}

#[test]
fn test_interleave_modifiers() {
    assert_actions(
        indoc! {"
        keymap:
          - remap:
              M-f: C-right
        "},
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTALT, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_F, KeyValue::Press)),
        ],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTALT, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTALT, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_RIGHT, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_RIGHT, KeyValue::Release)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTALT, KeyValue::Press)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
        ],
    )
}

#[test]
fn test_exact_match_true() {
    assert_actions(
        indoc! {"
        keymap:
          - exact_match: true
            remap:
              M-f: C-right
        "},
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTALT, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTSHIFT, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_F, KeyValue::Press)),
        ],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTALT, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTSHIFT, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_F, KeyValue::Press)),
        ],
    )
}

#[test]
fn test_exact_match_false() {
    assert_actions(
        indoc! {"
        keymap:
          - exact_match: false
            remap:
              M-f: C-right
        "},
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTALT, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTSHIFT, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_F, KeyValue::Press)),
        ],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTALT, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTSHIFT, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTALT, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_RIGHT, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_RIGHT, KeyValue::Release)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTALT, KeyValue::Press)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
        ],
    )
}

#[test]
fn test_exact_match_default() {
    assert_actions(
        indoc! {"
        keymap:
          - remap:
              M-f: C-right
        "},
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTALT, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTSHIFT, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_F, KeyValue::Press)),
        ],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTALT, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTSHIFT, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTALT, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_RIGHT, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_RIGHT, KeyValue::Release)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTALT, KeyValue::Press)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
        ],
    )
}

#[test]
fn test_exact_match_true_nested() {
    assert_actions(
        indoc! {"
        keymap:
          - exact_match: true
            remap:
              C-x:
                remap:
                  h: C-a
        "},
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_X, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_X, KeyValue::Release)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTSHIFT, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_H, KeyValue::Press)),
        ],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_X, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTSHIFT, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_H, KeyValue::Press)),
        ],
    )
}

#[test]
fn test_exact_match_false_nested() {
    assert_actions(
        indoc! {"
        keymap:
          - exact_match: false
            remap:
              C-x:
                remap:
                  h: C-a
        "},
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_X, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_X, KeyValue::Release)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTSHIFT, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_H, KeyValue::Press)),
        ],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_X, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTSHIFT, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_A, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_A, KeyValue::Release)),
            Action::Delay(Duration::from_nanos(0)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
        ],
    )
}

#[test]
fn test_application_override() {
    let config = indoc! {"
        keymap:

          - name: firefox
            application:
              only: [firefox]
            remap:
              a: C-c

          - name: generic
            remap:
              a: C-b
    "};

    assert_actions(
        config,
        vec![Event::KeyEvent(
            get_input_device_info(),
            KeyEvent::new(Key::KEY_A, KeyValue::Press),
        )],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_B, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_B, KeyValue::Release)),
            Action::Delay(Duration::from_nanos(0)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
        ],
    );

    assert_actions_with_current_application(
        config,
        Some(String::from("firefox")),
        vec![Event::KeyEvent(
            get_input_device_info(),
            KeyEvent::new(Key::KEY_A, KeyValue::Press),
        )],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_C, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_C, KeyValue::Release)),
            Action::Delay(Duration::from_nanos(0)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
        ],
    );
}

#[test]
fn test_device_override() {
    let config = indoc! {"
        keymap:

          - name: event1
            device:
              only: [event1]
            remap:
              a: C-c

          - name: event0
            remap:
              a: C-b
    "};

    assert_actions(
        config,
        vec![Event::KeyEvent(
            InputDeviceInfo {
                name: "Some Device",
                path: &Path::new("/dev/input/event0"),
            },
            KeyEvent::new(Key::KEY_A, KeyValue::Press),
        )],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_B, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_B, KeyValue::Release)),
            Action::Delay(Duration::from_nanos(0)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
        ],
    );

    assert_actions(
        config,
        vec![Event::KeyEvent(
            InputDeviceInfo {
                name: "Other Device",
                path: &Path::new("/dev/input/event1"),
            },
            KeyEvent::new(Key::KEY_A, KeyValue::Press),
        )],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_C, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_C, KeyValue::Release)),
            Action::Delay(Duration::from_nanos(0)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
        ],
    );
}

#[test]
fn test_merge_remaps() {
    let config = indoc! {"
        keymap:
          - remap:
              C-x:
                remap:
                  h: C-a
          - remap:
              C-x:
                remap:
                  k: C-w
    "};

    assert_actions(
        config,
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_X, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_X, KeyValue::Release)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_H, KeyValue::Press)),
        ],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_X, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_A, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_A, KeyValue::Release)),
            Action::Delay(Duration::from_nanos(0)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
        ],
    );

    assert_actions(
        config,
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_X, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_X, KeyValue::Release)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_K, KeyValue::Press)),
        ],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_X, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_W, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_W, KeyValue::Release)),
            Action::Delay(Duration::from_nanos(0)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
        ],
    )
}

#[test]
fn test_merge_remaps_with_override() {
    let config = indoc! {"
        keymap:
          - remap:
              C-x:
                remap:
                  h: C-a
          - remap:
              C-x:
                remap:
                  h: C-b
                  c: C-q
    "};

    assert_actions(
        config,
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_X, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_X, KeyValue::Release)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_H, KeyValue::Press)),
        ],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_X, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_A, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_A, KeyValue::Release)),
            Action::Delay(Duration::from_nanos(0)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
        ],
    );

    assert_actions(
        config,
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_X, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_X, KeyValue::Release)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_C, KeyValue::Press)),
        ],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_X, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_Q, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_Q, KeyValue::Release)),
            Action::Delay(Duration::from_nanos(0)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
        ],
    )
}

#[test]
fn test_mixing_keypress_and_remap_in_keymap_action() {
    // KEY_D will be emitted, and the remap will be used for next key press.
    assert_actions(
        indoc! {"
        keymap:
          - remap:
              f12:
                - d
                - remap:
                    a: b
        "},
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_F12, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_F12, KeyValue::Release)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_A, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_A, KeyValue::Release)),
        ],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_D, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_D, KeyValue::Release)),
            Action::Delay(Duration::from_nanos(0)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_F12, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_B, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_B, KeyValue::Release)),
            Action::Delay(Duration::from_nanos(0)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_A, KeyValue::Release)),
        ],
    )
}

#[test]
fn test_mixing_no_keypress_and_remap_in_keymap_action() {
    // The first match stops the search for matches. So the last remap isn't used.
    assert_actions(
        indoc! {"
        keymap:
          - remap:
              f12: []
          - remap:
              f12:
                - remap:
                    a: b
        "},
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_F12, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_F12, KeyValue::Release)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_A, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_A, KeyValue::Release)),
        ],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_F12, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_A, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_A, KeyValue::Release)),
        ],
    )
}

#[test]
fn test_no_keymap_action() {
    assert_actions(
        indoc! {"
        keymap:
          - remap:
              f12: []
        "},
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_F12, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_F12, KeyValue::Release)),
        ],
        vec![
            //This is just release, so the key is not emitted.
            Action::KeyEvent(KeyEvent::new(Key::KEY_F12, KeyValue::Release)),
        ],
    );

    //Same test with the null keyword
    assert_actions(
        indoc! {"
        keymap:
          - remap:
              f12: null
        "},
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_F12, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_F12, KeyValue::Release)),
        ],
        vec![Action::KeyEvent(KeyEvent::new(Key::KEY_F12, KeyValue::Release))],
    )
}

#[test]
fn test_any_key() {
    assert_actions(
        indoc! {"
        keymap:
          - remap:
              a: b
              ANY: null
        "},
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_A, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_A, KeyValue::Release)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_C, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_C, KeyValue::Release)),
        ],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_B, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_B, KeyValue::Release)),
            Action::Delay(Duration::from_nanos(0)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_A, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_C, KeyValue::Release)),
        ],
    );
}

#[test]
fn test_any_key_doesnt_match_modifier_1() {
    assert_actions(
        indoc! {"
        keymap:
          - remap:
              ANY: null
        "},
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
        ],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
        ],
    );
}

#[test]
fn test_any_key_doesnt_match_modifier_2() {
    assert_actions(
        indoc! {"
        keymap:
          - remap:
              Shift-ANY: null
        "},
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTSHIFT, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTSHIFT, KeyValue::Release)),
        ],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTSHIFT, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTSHIFT, KeyValue::Release)),
        ],
    );
}

#[test]
fn test_terminal_modifier() {
    assert_actions(
        indoc! {"
        keymap:
          - remap:
              c_l: end
        "},
        vec![Event::KeyEvent(
            get_input_device_info(),
            KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press),
        )],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_END, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_END, KeyValue::Release)),
            Action::Delay(Duration::from_nanos(0)),
            Action::Delay(Duration::from_nanos(0)),
        ],
    )
}

#[test]
fn test_terminal_modifier_with_other_modifier() {
    assert_actions(
        indoc! {"
        keymap:
          - remap:
              c_l-c_r: end
        "},
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_RIGHTCTRL, KeyValue::Press)),
        ],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_END, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_END, KeyValue::Release)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::Delay(Duration::from_nanos(0)),
        ],
    )
}

#[test]
fn test_terminal_modifier_sends_other_modifier_combo() {
    assert_actions(
        indoc! {"
        keymap:
          - remap:
              alt_l-alt_r: c-x
        "},
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTALT, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_RIGHTALT, KeyValue::Press)),
        ],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTALT, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTALT, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_X, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_X, KeyValue::Release)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTALT, KeyValue::Press)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Release)),
        ],
    )
}

#[test]
fn test_terminal_modifier_sends_same_modifier_combo() {
    assert_actions(
        indoc! {"
        keymap:
          - remap:
              c_r-c_l: c-x
        "},
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_RIGHTCTRL, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTCTRL, KeyValue::Press)),
        ],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_RIGHTCTRL, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_X, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_X, KeyValue::Release)),
            Action::Delay(Duration::from_nanos(0)),
            Action::Delay(Duration::from_nanos(0)),
        ],
    )
}

#[test]
fn test_terminal_modifier_with_exact_match() {
    assert_actions(
        indoc! {"
        keymap:
          - exact_match: true
            remap:
              shift_r: c
              win_l-shift_r: k
        "},
        vec![
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_LEFTMETA, KeyValue::Press)),
            Event::KeyEvent(get_input_device_info(), KeyEvent::new(Key::KEY_RIGHTSHIFT, KeyValue::Press)),
        ],
        vec![
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTMETA, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTMETA, KeyValue::Release)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_K, KeyValue::Press)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_K, KeyValue::Release)),
            Action::Delay(Duration::from_nanos(0)),
            Action::KeyEvent(KeyEvent::new(Key::KEY_LEFTMETA, KeyValue::Press)),
            Action::Delay(Duration::from_nanos(0)),
        ],
    )
}

fn assert_actions(config_yaml: &str, events: Vec<Event>, actions: Vec<Action>) {
    assert_actions_with_current_application(config_yaml, None, events, actions);
}

fn assert_actions_with_current_application(
    config_yaml: &str,
    current_application: Option<String>,
    events: Vec<Event>,
    actions: Vec<Action>,
) {
    let timer = TimerFd::new(ClockId::CLOCK_MONOTONIC, TimerFlags::empty()).unwrap();
    let mut config: Config = serde_yaml::from_str(config_yaml).unwrap();
    config.keymap_table = build_keymap_table(&config.keymap);
    let mut event_handler = EventHandler::new(
        timer,
        "default",
        Duration::from_micros(0),
        WMClient::new("static", Box::new(StaticClient { current_application })),
    );
    let mut actual: Vec<Action> = vec![];

    actual.append(&mut event_handler.on_events(&events, &config).unwrap());

    assert_eq!(format!("{:?}", actions), format!("{:?}", actual));
}
