use super::*;

#[test]
fn mouse_reporting_follows_the_setting() {
    assert!(enter_sequence(true).ends_with(MOUSE_ON));
    assert!(!enter_sequence(false).contains("?1000h"));
    assert!(ALT_OFF.starts_with(MOUSE_OFF) && RESTORE.starts_with(MOUSE_OFF));
    let mut cfg = Config::default();
    assert!(cfg.mouse, "mouse should default on");
    cfg.set("mouse", "off").unwrap();
    assert!(!cfg.mouse);
}

#[test]
fn session_keys_decode_raw_and_disambiguated() {
    let mut buf = b"\x05\x0f\x0e\x10\x1b[101;5u\x1b[111;5u\x1b[110;5u\x1b[112;5u".to_vec();
    let keys = decode(&mut buf);
    let letters: Vec<char> = keys
        .iter()
        .map(|k| match k {
            Key::Ctrl(c) => *c,
            other => panic!("not a control key: {other:?}"),
        })
        .collect();
    assert_eq!(letters, ['e', 'o', 'n', 'p', 'e', 'o', 'n', 'p']);
}

#[test]
fn decodes_the_wheel_and_a_left_press() {
    let mut buf = b"\x1b[<64;10;5M\x1b[<65;10;5M\x1b[<0;3;4M\x1b[<0;3;4m\x1b[<68;1;1M".to_vec();
    let keys = decode(&mut buf);
    assert!(matches!(keys[0], Key::WheelUp), "{keys:?}");
    assert!(matches!(keys[1], Key::WheelDown), "{keys:?}");
    assert!(matches!(keys[2], Key::Click(3, 4)), "press: {keys:?}");
    assert!(matches!(keys[3], Key::Release(3, 4)), "release: {keys:?}");
    assert!(matches!(keys[4], Key::WheelUp), "shift+wheel: {keys:?}");
    assert!(buf.is_empty());
}

#[test]
fn the_wheel_scrolls_the_transcript_and_moves_a_picker() {
    let mut a = long_app();
    let (tx, _rx) = channel();
    let tail = a.frame(60, 8);
    a.handle_key(Key::WheelUp, &tx);
    assert!(!a.follow, "wheel up did not detach");
    assert_ne!(a.frame(60, 8), tail);
    for _ in 0..20 {
        a.handle_key(Key::WheelDown, &tx);
        a.frame(60, 8);
    }
    assert!(a.follow, "scrolling to the end did not resume following");

    let mut b = app();
    open_many(&mut b, 5);
    b.handle_key(Key::WheelDown, &tx);
    assert_eq!(open_picker(&b).index, 1);
    b.handle_key(Key::WheelUp, &tx);
    assert_eq!(open_picker(&b).index, 0);
}

#[test]
fn a_click_does_not_answer_a_prompt() {
    let mut a = picker();
    let (tx, rx) = channel();
    a.handle_key(Key::Char('d'), &tx);
    a.handle_key(Key::Unknown, &tx);
    a.handle_key(Key::WheelDown, &tx);
    assert!(matches!(a.pending, Some(Pending::ConfirmDelete { .. })), "the question closed");
    assert!(rx.try_recv().is_err());
}

#[test]
fn decodes_arrows_and_text() {
    let mut buf = b"\x1b[Aa\x7f\r".to_vec();
    let keys = decode(&mut buf);
    assert_eq!(keys.len(), 4);
    assert!(matches!(keys[0], Key::Up));
    assert!(matches!(keys[1], Key::Char('a')));
    assert!(matches!(keys[2], Key::Backspace));
    assert!(matches!(keys[3], Key::Enter));
    assert!(buf.is_empty());
}

#[test]
fn decodes_utf8() {
    let mut buf = "é".as_bytes().to_vec();
    assert!(matches!(decode(&mut buf)[0], Key::Char('é')));
}

#[test]
fn alt_enter_is_newline() {
    let mut buf = b"\x1b\r".to_vec();
    assert!(matches!(decode(&mut buf)[0], Key::Newline));
}

#[test]
fn shift_enter_is_newline() {
    let mut buf = b"\x1b[13;2u".to_vec();
    let keys = decode(&mut buf);
    assert_eq!(keys.len(), 1);
    assert!(matches!(keys[0], Key::Newline));
    assert!(buf.is_empty());
}

#[test]
fn disambiguated_enter_and_escape() {
    let mut buf = b"\x1b[13u\x1b[27u".to_vec();
    let keys = decode(&mut buf);
    assert_eq!(keys.len(), 2);
    assert!(matches!(keys[0], Key::Enter));
    assert!(matches!(keys[1], Key::Esc));
}

#[test]
fn modified_arrow_takes_no_parameters_as_text() {
    // Shift selects; any other modifier keeps the plain arrow.
    let mut buf = b"\x1b[1;5A".to_vec();
    let keys = decode(&mut buf);
    assert_eq!(keys.len(), 1);
    assert!(matches!(keys[0], Key::Up));
}

#[test]
fn a_selection_is_extended_replaced_and_cleared() {
    let (tx, _cmds) = channel();
    let draft = |text: &str| {
        let mut a = app();
        a.input = text.into();
        a.cursor = text.len();
        a
    };
    // Replace a word by typing.
    let mut a = draft("make it red");
    for _ in 0..3 {
        a.handle_key(Key::Select(Move::Left), &tx);
    }
    assert_eq!(a.selection(), Some((8, 11)));
    for c in "blue".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    assert_eq!((a.input.as_str(), a.cursor), ("make it blue", 12));
    // Backspace removes it; the caret sits where it began.
    let mut a = draft("make it red");
    a.handle_key(Key::Select(Move::Left), &tx);
    a.handle_key(Key::Select(Move::Left), &tx);
    a.handle_key(Key::Backspace, &tx);
    assert_eq!((a.input.as_str(), a.cursor), ("make it r", 9));
    // A plain move clears it and leaves the text alone.
    let mut a = draft("make it red");
    a.handle_key(Key::Select(Move::Home), &tx);
    a.handle_key(Key::Right, &tx);
    assert_eq!((a.input.as_str(), a.selection()), ("make it red", None));
    // Ctrl+V and a newline replace it too.
    let mut a = draft("make it red");
    a.clip = "X".into();
    a.handle_key(Key::Select(Move::Left), &tx);
    a.handle_key(Key::Ctrl('v'), &tx);
    assert_eq!(a.input, "make it reX");
    a.handle_key(Key::Select(Move::Left), &tx);
    a.handle_key(Key::Newline, &tx);
    assert_eq!(a.input, "make it re\n");
    // Across rows, without recalling history.
    let mut a = draft("ab\ncd");
    a.history = vec!["old".into()];
    a.handle_key(Key::Select(Move::Up), &tx);
    assert_eq!((a.input.as_str(), a.selection()), ("ab\ncd", Some((2, 5))));
    a.handle_key(Key::Select(Move::Up), &tx);
    assert_eq!(a.input, "ab\ncd", "Shift+Up recalled history");
}

#[test]
fn a_selection_is_drawn_reversed() {
    let (tx, _cmds) = channel();
    // Colour off: reverse video still shows it, as it does the caret.
    let mut a = app();
    a.input = "make it red".into();
    a.cursor = a.input.len();
    for _ in 0..3 {
        a.handle_key(Key::Select(Move::Left), &tx);
    }
    let out = a.frame(54, 10);
    assert!(out.contains("make it \x1b[7m \x1b[0m\x1b[7mred\x1b[27m"), "{out:?}");
    assert!(!out.contains('\u{e000}') && !out.contains('\u{e001}'), "marks leaked: {out:?}");
    // Across rows: the end of the first and all of the second.
    let mut b = app();
    b.input = "ab\ncd".into();
    b.cursor = 5;
    b.handle_key(Key::Select(Move::Up), &tx);
    let out = b.frame(54, 10);
    assert!(out.contains("> ab\x1b[7m \x1b[0m"), "{out:?}");
    assert!(out.contains("  \x1b[7mcd\x1b[27m"), "{out:?}");
    // A long selected draft still fits the width.
    let mut c = app();
    c.input = "x".repeat(120);
    c.cursor = 120;
    for _ in 0..80 {
        c.handle_key(Key::Select(Move::Left), &tx);
    }
    let row = strip(&c.frame(54, 10)).lines().find(|l| l.starts_with("> ")).unwrap().trim_end_matches('\r').to_string();
    assert!(row.chars().count() <= 54, "{row:?}");
}

#[test]
fn ctrl_v_attaches_a_clipboard_image_or_pastes_the_copy() {
    let (tx, cmds) = channel();
    let dir = scratch("paste");
    let png = dir.join("shot.png");
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.extend_from_slice(&[0u8; 16]);
    std::fs::write(&png, &bytes).unwrap();
    // An image on the clipboard is attached with the usual marker.
    let mut a = app();
    a.paste_cmd = Some(format!("cat '{}'", png.display()));
    a.input = "keep".into();
    a.cursor = 4;
    assert!(a.paste_image(&tx, &dir));
    assert!(a.entries.last().unwrap().text.starts_with("[image pasted.png ·"), "{:?}", a.entries.last().unwrap().text);
    assert!(matches!(cmds.try_recv(), Ok(Cmd::Attach(v)) if v.len() == 1 && v[0].name == "pasted.png"));
    assert_eq!(a.input, "keep", "the draft changed");
    // Text on the clipboard: Ctrl+V pastes imp's last copy instead.
    let mut b = app();
    b.paste_cmd = Some("printf 'some text'".into());
    b.clip = "copied".into();
    b.handle_key(Key::Ctrl('v'), &tx);
    assert_eq!(b.input, "copied");
    assert!(cmds.try_recv().is_err(), "text was attached");
    // No clipboard: one note, however often Ctrl+V is pressed.
    let mut c = app();
    c.clip = "x".into();
    c.handle_key(Key::Ctrl('v'), &tx);
    c.handle_key(Key::Ctrl('v'), &tx);
    assert_eq!(c.input, "xx");
    assert_eq!(c.entries.iter().filter(|e| e.text.contains("cannot paste images")).count(), 1);
}

#[test]
fn enter_queues_during_a_turn_and_ctrl_q_does_nothing() {
    let (tx, _cmds) = channel();
    let mut b = Vec::from(&b"\x11"[..]);
    assert_eq!(decode(&mut b), vec![Key::Ctrl('q')]);
    let mut a = app();
    a.status.streaming = true;
    a.input = "also add tests".into();
    a.cursor = a.input.len();
    a.handle_key(Key::Enter, &tx);
    assert_eq!(a.queued, vec!["also add tests"], "Enter did not queue");
    assert!(a.input.is_empty());
    let shown = strip(&a.frame(60, 10));
    assert!(shown.contains("\r\n» "), "the glyph is not » during a turn: {shown:?}");
    a.input = "draft".into();
    a.cursor = a.input.len();
    a.handle_key(Key::Ctrl('q'), &tx);
    assert_eq!(a.queued, vec!["also add tests"], "Ctrl+Q changed the queue");
    assert_eq!(a.input, "draft", "Ctrl+Q changed the draft");
}

#[test]
fn ctrl_s_steers_during_a_turn() {
    let (tx, cmds) = channel();
    let mut b = Vec::from(&b"\x13"[..]);
    assert_eq!(decode(&mut b), vec![Key::Ctrl('s')]);
    let mut a = app();
    a.status.streaming = true;
    a.begin_task();
    a.input = "use pnpm, not npm".into();
    a.cursor = a.input.len();
    a.handle_key(Key::Ctrl('s'), &tx);
    assert_eq!(*a.steer.lock().unwrap(), vec!["use pnpm, not npm"]);
    assert_eq!(a.entries.last().unwrap().text, "use pnpm, not npm (steered)");
    assert!(a.queued.is_empty(), "a steered message was queued");
    assert!(a.input.is_empty());
    assert!(cmds.try_recv().is_err(), "a steered message was sent as a turn");
}

#[test]
fn ctrl_s_sends_when_idle_and_does_nothing_on_an_empty_draft() {
    let (mut a, cmds, _ev) = worker_app();
    let tx = a.worker.as_ref().unwrap().0.clone();
    a.handle_key(Key::Ctrl('s'), &tx);
    assert!(cmds.try_recv().is_err(), "an empty draft was sent");
    assert!(!a.status.streaming, "an empty draft started a turn");
    a.input = "hi".into();
    a.cursor = a.input.len();
    a.handle_key(Key::Ctrl('s'), &tx);
    assert!(matches!(cmds.try_recv(), Ok(Cmd::Prompt(t)) if t == "hi"), "Ctrl+S did not send when idle");
    assert!(a.status.streaming, "the turn did not start");
}

/// An app whose worker is a pair of test channels.
fn worker_app() -> (App, Receiver<Cmd>, Sender<Ev>) {
    let mut a = app();
    let (cmd_tx, cmd_rx) = channel();
    let (ev_tx, ev_rx) = channel();
    a.worker = Some((cmd_tx, ev_rx, Default::default()));
    (a, cmd_rx, ev_tx)
}

#[test]
fn queued_messages_wait_for_the_turn_to_end() {
    let (mut a, cmds, _ev) = worker_app();
    let tx = a.worker.as_ref().unwrap().0.clone();
    a.status.streaming = true;
    a.begin_task();
    a.status.steps = 4;
    a.input = "also add tests".into();
    a.cursor = a.input.len();
    a.handle_key(Key::Enter, &tx);
    assert_eq!(a.queued, vec!["also add tests"]);
    assert!(!a.entries.iter().any(|e| e.text == "also add tests"), "queued text reached the transcript");
    assert!(cmds.try_recv().is_err(), "a queued message was sent");
    assert_eq!(a.status.steps, 4, "the meter restarted");
    assert!(strip(&a.frame(60, 12)).contains("⋯ also add tests"));
    // The turn ends: the queued message starts the next one.
    a.apply(Ev::Done(None));
    assert!(matches!(cmds.try_recv(), Ok(Cmd::Prompt(t)) if t == "also add tests"));
    assert!(a.queued.is_empty() && a.status.streaming);
    // A stopped turn leaves the queue waiting.
    a.queued.push("later".into());
    a.cancel.store(true, Ordering::Relaxed);
    a.apply(Ev::Done(None));
    assert_eq!(a.queued, vec!["later"]);
    assert!(cmds.try_recv().is_err());
    // Steer mode, until steering lands, still keeps the meter.
    let (mut b, _c, _e) = worker_app();
    let tx = b.worker.as_ref().unwrap().0.clone();
    b.status.streaming = true;
    b.begin_task();
    b.status.steps = 3;
    b.input = "more".into();
    b.cursor = 4;
    b.handle_key(Key::Enter, &tx);
    assert_eq!(b.status.steps, 3, "the meter restarted");
}

#[test]
fn queued_messages_are_walked_and_edited_in_place() {
    let (mut a, cmds, _ev) = worker_app();
    let tx = a.worker.as_ref().unwrap().0.clone();
    a.status.streaming = true;
    a.history = vec!["old".into()];
    a.queued = vec!["first".into(), "second".into()];
    // Newest first, holding the turn, then older, then history.
    a.handle_key(Key::Up, &tx);
    assert_eq!((a.queue_at, a.input.as_str()), (Some(1), "second"));
    assert!(a.hold.load(Ordering::SeqCst), "the turn is not held");
    let shown = strip(&a.frame(70, 14));
    assert!(shown.contains("editing 2/2 · Enter save · ^S steer · ^X del"), "{shown:?}");
    assert!(a.status_line(120).contains("paused · editing queue"));
    a.handle_key(Key::Up, &tx);
    assert_eq!((a.queue_at, a.input.as_str()), (Some(0), "first"));
    a.handle_key(Key::Up, &tx);
    assert_eq!((a.queue_at, a.input.as_str(), a.hist_pos), (None, "old", Some(0)));
    assert!(!a.hold.load(Ordering::SeqCst), "the hold outlived the edit");
    // Edit in place with Enter.
    a.input.clear();
    a.cursor = 0;
    a.hist_pos = None;
    a.handle_key(Key::Up, &tx);
    a.input = "second, edited".into();
    a.cursor = a.input.len();
    a.handle_key(Key::Enter, &tx);
    assert_eq!(a.queued, vec!["first", "second, edited"]);
    assert!(a.queue_at.is_none() && a.input.is_empty());
    assert!(cmds.try_recv().is_err(), "saving sent something");
    // Emptied, it is removed.
    a.handle_key(Key::Up, &tx);
    a.input.clear();
    a.cursor = 0;
    a.handle_key(Key::Enter, &tx);
    assert_eq!(a.queued, vec!["first"]);
    // Down past the newest returns to an empty draft.
    a.handle_key(Key::Up, &tx);
    a.handle_key(Key::Down, &tx);
    assert!(a.queue_at.is_none() && a.input.is_empty() && a.queued == vec!["first"]);
    // A turn ending mid-edit waits for the edit, then goes on.
    a.handle_key(Key::Up, &tx);
    a.apply(Ev::Done(None));
    assert!(cmds.try_recv().is_err(), "sent while being edited");
    a.handle_key(Key::Enter, &tx);
    assert!(matches!(cmds.try_recv(), Ok(Cmd::Prompt(t)) if t == "first"));
}

#[test]
fn enter_no_longer_steers_a_running_turn() {
    let (mut a, cmds, _ev) = worker_app();
    let tx = a.worker.as_ref().unwrap().0.clone();
    a.status.streaming = true;
    a.input = "use pnpm, not npm".into();
    a.cursor = a.input.len();
    a.handle_key(Key::Enter, &tx);
    assert!(a.steer.lock().unwrap().is_empty(), "Enter steered");
    assert_eq!(a.queued, vec!["use pnpm, not npm".to_string()]);
    assert!(cmds.try_recv().is_err(), "a queued message was sent as a turn");
    // Steered too late: the next turn carries it.
    a.apply(Ev::Done(None));
    assert!(matches!(cmds.try_recv(), Ok(Cmd::Prompt(t)) if t == "use pnpm, not npm"));
    assert!(a.status.streaming && a.steer.lock().unwrap().is_empty());
    // After a stop it waits at the front of the queue.
    a.queued = vec!["queued".into()];
    a.steer.lock().unwrap().push("late".into());
    a.cancel.store(true, Ordering::Relaxed);
    a.apply(Ev::Done(None));
    assert_eq!(a.queued, vec!["late", "queued"]);
    assert!(cmds.try_recv().is_err());
}

#[test]
fn ctrl_s_steers_the_open_queued_message() {
    let (mut a, cmds, _ev) = worker_app();
    let tx = a.worker.as_ref().unwrap().0.clone();
    a.status.streaming = true;
    a.begin_task();
    a.queued = vec!["first".into(), "second".into()];
    a.handle_key(Key::Up, &tx);
    a.input = "second, but terser".into();
    a.cursor = a.input.len();
    a.handle_key(Key::Ctrl('s'), &tx);
    assert_eq!(a.queued, vec!["first"]);
    assert_eq!(*a.steer.lock().unwrap(), vec!["second, but terser"]);
    assert_eq!(a.entries.last().unwrap().text, "second, but terser (steered)");
    assert_eq!(a.queue_at, None);
    assert!(a.input.is_empty(), "the composer kept the steered text");
    assert!(!a.hold.load(Ordering::SeqCst), "the hold outlived the edit");
    assert!(cmds.try_recv().is_err(), "a steered message was sent as a turn");
}

#[test]
fn ctrl_s_sends_the_open_message_when_no_turn_runs() {
    let (mut a, cmds, _ev) = worker_app();
    let tx = a.worker.as_ref().unwrap().0.clone();
    a.queued = vec!["only".into()];
    a.handle_key(Key::Up, &tx);
    let out = strip(&a.frame(70, 14));
    assert!(out.contains("^S send"), "{out:?}");
    a.handle_key(Key::Ctrl('s'), &tx);
    assert!(a.queued.is_empty());
    assert!(matches!(cmds.try_recv(), Ok(Cmd::Prompt(t)) if t == "only"));
    assert_eq!(a.queue_at, None);
    assert!(a.status.streaming, "the turn did not start");
}

#[test]
fn ctrl_x_deletes_and_opens_the_next_message() {
    let (mut a, _cmds, _ev) = worker_app();
    let tx = a.worker.as_ref().unwrap().0.clone();
    a.status.streaming = true;
    a.queued = vec!["first".into(), "second".into()];
    a.handle_key(Key::Up, &tx);
    a.handle_key(Key::Up, &tx);
    assert_eq!((a.queue_at, a.input.as_str()), (Some(0), "first"));
    a.handle_key(Key::Ctrl('x'), &tx);
    assert_eq!(a.queued, vec!["second"]);
    assert_eq!((a.queue_at, a.input.as_str()), (Some(0), "second"));
    assert!(a.hold.load(Ordering::SeqCst), "the next message is not held");
    a.handle_key(Key::Ctrl('x'), &tx);
    assert!(a.queued.is_empty());
    assert_eq!(a.queue_at, None);
    assert!(!a.hold.load(Ordering::SeqCst));
}

#[test]
fn ctrl_c_closes_the_queue_editor_unchanged() {
    let (mut a, _cmds, _ev) = worker_app();
    let tx = a.worker.as_ref().unwrap().0.clone();
    a.status.streaming = true;
    a.queued = vec!["keep me".into()];
    a.handle_key(Key::Up, &tx);
    a.input = "changed".into();
    a.handle_key(Key::Ctrl('c'), &tx);
    assert_eq!(a.queued, vec!["keep me"], "the edit was written or dropped");
    assert_eq!(a.queue_at, None);
    assert!(!a.quit, "Ctrl+C quit from the queue editor");
    assert!(!a.hold.load(Ordering::SeqCst));
}

#[test]
fn leaving_the_editor_by_up_after_the_turn_ends_releases_the_queue() {
    let (mut a, cmds, _ev) = worker_app();
    let tx = a.worker.as_ref().unwrap().0.clone();
    a.status.streaming = true;
    a.history = vec!["old".into()];
    a.queued = vec!["first".into(), "second".into()];
    a.handle_key(Key::Up, &tx);
    assert_eq!(a.queue_at, Some(1));
    a.apply(Ev::Done(None));
    assert!(a.queue_due, "the ended turn did not wait for the editor");
    // Up once reaches the older message; the next Up leaves for history.
    a.handle_key(Key::Up, &tx);
    assert_eq!((a.queue_at, a.input.as_str()), (Some(0), "first"));
    assert!(cmds.try_recv().is_err(), "the queue sent while still editing");
    a.handle_key(Key::Up, &tx);
    assert!(matches!(cmds.try_recv(), Ok(Cmd::Prompt(t)) if t == "first"));
    assert_eq!(a.queued, vec!["second"], "the wrong message went");
    assert!(!a.queue_due);
}

#[test]
fn a_stale_queue_index_cannot_panic() {
    let (mut a, _cmds, _ev) = worker_app();
    let tx = a.worker.as_ref().unwrap().0.clone();
    a.status.streaming = true;
    a.queued = vec!["one".into()];
    a.queue_at = Some(5);
    a.input = "edit".into();
    a.handle_key(Key::Enter, &tx);
    assert_eq!(a.queued, vec!["one"], "the queue changed");
    assert_eq!(a.queue_at, None);
}

#[test]
fn the_edit_rule_fits_and_names_every_key() {
    let (mut a, _cmds, _ev) = worker_app();
    let tx = a.worker.as_ref().unwrap().0.clone();
    a.status.streaming = true;
    a.queued = vec!["only".into()];
    a.handle_key(Key::Up, &tx);
    let running = strip(&a.frame(54, 14));
    assert!(running.contains("editing 1/1 · Enter save · ^S steer · ^X del"), "{running:?}");
    a.status.streaming = false;
    let idle = strip(&a.frame(54, 14));
    assert!(idle.contains("editing 1/1 · Enter save · ^S send · ^X del"), "{idle:?}");
}

#[test]
fn an_idle_queue_points_at_up() {
    let mut a = app();
    a.queued = vec!["waiting".into()];
    let out = strip(&a.frame(54, 12));
    assert!(out.contains("↑ edit queue"), "{out:?}");
    // While a turn runs the rule still says what Enter does.
    a.status.streaming = true;
    let out = strip(&a.frame(54, 12));
    assert!(!out.contains("↑ edit queue") && !out.contains("^Q"), "{out:?}");
}

#[test]
fn deleting_a_queued_message_is_reported() {
    let (mut a, _cmds, _ev) = worker_app();
    let tx = a.worker.as_ref().unwrap().0.clone();
    a.status.streaming = true;
    a.queued = vec!["first".into(), "second".into()];
    a.handle_key(Key::Up, &tx);
    a.handle_key(Key::Up, &tx);
    a.handle_key(Key::Ctrl('x'), &tx);
    assert!(
        a.entries.iter().any(|e| e.text == "queue: deleted 1"),
        "no deletion line"
    );
    a.input.clear();
    a.handle_key(Key::Enter, &tx);
    assert!(a.queued.is_empty());
    assert!(
        a.entries.iter().filter(|e| e.text == "queue: deleted 1").count() >= 2,
        "emptying did not report"
    );
}

#[test]
fn an_open_queued_message_holds_the_turn_and_esc_still_stops_it() {
    let (mut a, _cmds, _ev) = worker_app();
    let tx = a.worker.as_ref().unwrap().0.clone();
    a.status.streaming = true;
    a.queued = vec!["next".into()];
    a.handle_key(Key::Up, &tx);
    assert!(a.hold.load(Ordering::SeqCst));
    a.handle_key(Key::Esc, &tx);
    assert!(a.cancel.load(Ordering::Relaxed), "Esc did not stop the held turn");
    // Esc closes the editor: the hold is released and the message stays queued.
    assert!(!a.hold.load(Ordering::SeqCst), "the hold outlived the editor");
    assert_eq!(a.queued, vec!["next"]);
}

#[test]
fn ctrl_b_cycles_the_mode_at_once() {
    let (tx, _cmds) = channel();
    let mut a = app();
    a.status.streaming = true;
    let mut seen = Vec::new();
    for _ in 0..3 {
        a.handle_key(Key::Ctrl('b'), &tx);
        seen.push(a.cfg.live_permissions());
    }
    use crate::config::Permissions::*;
    assert_eq!(seen, vec![Auto, Yolo, Strict]);
    assert_eq!(a.entries.len(), 3, "one line per change");
    // Strict to normal, auto, then yolo turns the rule red.
    for _ in 0..3 {
        a.handle_key(Key::Ctrl('b'), &tx);
    }
    assert!(a.yolo());
}

#[test]
fn shift_tab_cycles_agents_even_with_a_draft() {
    let (tx, cmds) = channel();
    let mut a = app();
    a.agents = vec![("build".into(), String::new()), ("review".into(), String::new())];
    a.agent = "build".into();
    a.handle_key(Key::BackTab, &tx);
    assert_eq!(a.agent, "review");
    assert!(matches!(cmds.try_recv(), Ok(Cmd::SetAgent(n)) if n == "review"));
    // Tab never changes the agent: it completes.
    a.handle_key(Key::Tab, &tx);
    assert_eq!(a.agent, "review");
    assert!(cmds.try_recv().is_err());
    // Text in the draft: Shift+Tab still changes the agent.
    a.input = "fix".into();
    a.cursor = 3;
    a.handle_key(Key::BackTab, &tx);
    assert_eq!(a.agent, "build");
    assert!(matches!(cmds.try_recv(), Ok(Cmd::SetAgent(n)) if n == "build"));
    assert_eq!(a.input, "fix", "the draft is kept");
    // Mid-turn: the choice waits for the turn's end.
    a.input.clear();
    a.cursor = 0;
    a.status.streaming = true;
    a.handle_key(Key::BackTab, &tx);
    assert_eq!((a.agent.as_str(), a.agent_next.as_deref()), ("build", Some("review")));
    assert!(a.entries.is_empty(), "a switch wrote a transcript line");
    a.apply(Ev::Done(None));
    assert_eq!((a.agent.as_str(), a.agent_next.as_deref()), ("review", None));
    // One agent: Shift+Tab does nothing.
    let mut b = app();
    b.agents = vec![("build".into(), String::new())];
    b.agent = "build".into();
    b.handle_key(Key::BackTab, &tx);
    assert!(b.entries.is_empty());
}

#[test]
fn the_prompt_names_the_agent_and_the_status_line_the_mode() {
    let mut a = app();
    a.agents = vec![("build".into(), String::new())];
    a.agent = "build".into();
    assert!(strip(&a.frame(60, 10)).contains("\r\n> "), "one agent should give a bare prompt");
    a.agents.push(("plan".into(), String::new()));
    a.input = "x".repeat(70);
    a.cursor = a.input.len();
    let rows: Vec<String> = strip(&a.frame(60, 12)).split("\r\n").map(String::from).collect();
    let first = rows.iter().position(|r| r.starts_with("build> ")).expect("no agent before the prompt");
    assert!(rows[first + 1].starts_with("       x"), "the next row is not aligned: {rows:?}");
    a.agent_next = Some("plan".into());
    assert!(strip(&a.frame(60, 12)).contains("\r\nplan> "));
    assert!(a.status_line(80).starts_with("deepseek-flash · 0 tok"), "{}", a.status_line(80));
    a.cfg.set_permissions(crate::config::Permissions::Strict);
    assert!(a.status_line(80).starts_with("deepseek-flash · strict · 0 tok"), "{}", a.status_line(80));
    assert!(a.status_line(54).chars().count() <= 54);
}

#[test]
fn slash_keys_opens_the_keys_card() {
    let (tx, _cmds) = channel();
    let mut a = app();
    a.run_command("keys", &tx);
    assert!(matches!(&a.pending, Some(Pending::Pick(p)) if p.kind == PickKind::Keys));
    let out = strip(&a.frame(80, 30));
    assert!(out.contains("Ctrl+S") && out.contains("Shift+Tab"), "{out:?}");
}

#[test]
fn changed_words_stand_out_in_paired_lines() {
    let lines: Vec<String> = ["-let count = 3;", "+let count = 4;"].iter().map(|s| s.to_string()).collect();
    let marks = diff_marks(&lines, "");
    let changed = |i: usize| -> String {
        lines[i].chars().zip(&marks[i]).filter(|(_, m)| **m & CHANGED != 0).map(|(c, _)| c).collect()
    };
    assert_eq!((changed(0), changed(1)), ("3".to_string(), "4".to_string()));
    // An added run with nothing removed before it has no emphasis.
    let lone: Vec<String> = ["+one", "+two"].iter().map(|s| s.to_string()).collect();
    assert!(diff_marks(&lone, "").iter().flatten().all(|m| m & CHANGED == 0));
    // Drawn: the changed word bold on the stronger tint.
    let mut a = colour_app();
    a.push_diffs(lines.clone(), "");
    let out = a.frame(54, 10);
    let fg = a.palette.theme.rgb(crate::theme::Slot::Text);
    let [r, g, b] = crate::theme::tint_by(a.palette.theme.rgb(crate::theme::Slot::Success), fg, 2);
    assert!(out.contains(&format!("\x1b[48;2;{r};{g};{b}m\x1b[38;2;255;255;255m\x1b[1m4")), "{out:?}");
}

#[test]
fn diff_rows_carry_syntax_under_the_word_emphasis() {
    let mut a = colour_app();
    a.push_diffs(vec![r#"+let x = "hi"; // note"#.into()], "rs");
    let out = a.frame(60, 8);
    let t = a.palette.theme;
    let bg = crate::theme::tint(t.rgb(crate::theme::Slot::Success), t.rgb(crate::theme::Slot::Text));
    let on = |slot: crate::theme::Slot| {
        let [x, y, z] = t.rgb(slot);
        format!("\x1b[48;2;{};{};{}m\x1b[38;2;{x};{y};{z}m", bg[0], bg[1], bg[2])
    };
    use crate::theme::Slot::*;
    assert!(out.contains(&format!("{}\x1b[1mlet", on(Text))), "keyword: {out:?}");
    assert!(out.contains(&format!("{}\"hi\"", on(Success))), "string: {out:?}");
    // The muted colour is under 4.5:1 on the green band, so a comment
    // keeps only its italic.
    assert!(out.contains(&format!("{}\x1b[3m// note", on(Text))), "comment: {out:?}");
    // Plain text for an unknown extension.
    let mut b = colour_app();
    b.push_diffs(vec![r#"+let x = "hi";"#.into()], "txt");
    let plain = b.frame(60, 8);
    assert!(!plain.contains("\x1b[1mlet") && plain.contains(&format!("{}+let x = \"hi\";", on(Text))), "{plain:?}");
}

#[test]
fn a_faint_syntax_colour_falls_back_to_the_text_colour() {
    use crate::theme::{by_name, contrast, tint, Slot};
    let line = vec![r#"+let x = "hi";"#.to_string()];
    let string_fg = |name: &str| {
        let mut a = colour_app();
        a.palette = a.palette.with_theme(by_name(name).unwrap());
        a.push_diffs(line.clone(), "rs");
        let out = a.frame(60, 8);
        let t = a.palette.theme;
        let bg = tint(t.rgb(Slot::Success), t.rgb(Slot::Text));
        let at = out.find("\"hi\"").expect("string run");
        let before = &out[..at];
        let fg = &before[before.rfind("\x1b[38;2;").unwrap() + 7..];
        let rgb: Vec<u8> = fg.trim_end_matches('m').split(';').map(|v| v.parse().unwrap()).collect();
        ([rgb[0], rgb[1], rgb[2]], t.rgb(Slot::Success), t.rgb(Slot::Text), bg)
    };
    let (fg, success, text, bg) = string_fg("solarized-light");
    assert!(contrast(success, bg) < 4.5, "solarized-light is expected to be faint here");
    assert_eq!(fg, text, "solarized-light kept a faint string colour");
    let (fg, success, _, _) = string_fg("default");
    assert_eq!(fg, success, "default lost its string colour");
}

#[test]
fn slash_diff_shows_a_summary_then_styled_capped_diffs() {
    use crate::agent::FileDiff;
    let (tx, cmds) = channel();
    let mut a = app();
    a.run_command("diff", &tx);
    assert!(matches!(cmds.try_recv(), Ok(Cmd::Diff(args)) if args.is_empty()));
    a.run_command("diff git --staged", &tx);
    assert!(matches!(cmds.try_recv(), Ok(Cmd::Diff(args)) if args == "git --staged"));
    a.apply(Ev::Diff(Vec::new(), false));
    assert!(a.entries.last().unwrap().text.contains("changed no file"));
    let f = |name: &str, ext: &str, rows: Vec<String>, adds: usize, dels: usize| FileDiff {
        name: name.into(), ext: ext.into(), adds, dels, rows, untracked: false,
    };
    let many: Vec<String> = (0..250).map(|i| format!("+line {i}")).collect();
    let mut b = app();
    b.apply(Ev::Diff(vec![
        f("calc.py", "py", vec!["-    return a - b".into(), "+    return a * b".into()], 1, 1),
        f("long.txt", "txt", many, 250, 0),
        FileDiff { name: "big.bin".into(), ext: "bin".into(), adds: 0, dels: 0, rows: Vec::new(), untracked: true },
    ], false));
    let summary = &b.entries[0].text;
    assert!(summary.contains("calc.py   +1 −1") && summary.contains("long.txt  +250 −0"), "{summary}");
    assert!(summary.contains("big.bin   not diffed"), "{summary}");
    assert!(b.entries.iter().any(|e| e.text == "── calc.py"));
    let py = b.entries.iter().find(|e| e.text.contains("return a * b")).unwrap();
    assert_eq!(py.role, Role::Add);
    assert!(py.marks.iter().any(|m| m & !CHANGED == crate::syntax::KEYWORD), "no syntax marks");
    let shown = b.entries.iter().filter(|e| e.role == Role::Add || e.role == Role::Delete).count();
    assert_eq!(shown, 200, "the cap did not hold");
    assert!(b.entries.iter().any(|e| e.text == "… 52 more lines"));
    assert!(b.entries.last().unwrap().text.contains("not tracked"));
}

#[test]
fn words_move_delete_and_select() {
    let (tx, _cmds) = channel();
    let draft = |text: &str, at: usize| {
        let mut a = app();
        a.input = text.into();
        a.cursor = at;
        a
    };
    // Delete the word before the caret, either key.
    for key in [Key::DeleteWord(false), Key::Ctrl('w')] {
        let mut a = draft("run the tests", 13);
        a.handle_key(key, &tx);
        assert_eq!((a.input.as_str(), a.cursor), ("run the ", 8));
    }
    // Moves skip punctuation and spaces.
    let mut a = draft("foo(bar, baz)", 13);
    a.handle_key(Key::Word(Move::Left), &tx);
    assert_eq!(a.cursor, 9);
    a.handle_key(Key::Word(Move::Left), &tx);
    assert_eq!(a.cursor, 4);
    a.handle_key(Key::Word(Move::Right), &tx);
    assert_eq!(a.cursor, 7);
    // Delete forward.
    let mut a = draft("one two", 0);
    a.handle_key(Key::DeleteWord(true), &tx);
    assert_eq!(a.input, " two");
    // Replace a word by selecting it.
    let mut a = draft("use npm", 7);
    a.handle_key(Key::SelectWord(Move::Left), &tx);
    for c in "pnpm".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    assert_eq!(a.input, "use pnpm");
    // Multi-byte words stay whole.
    let mut a = draft("café crème", "café crème".len());
    a.handle_key(Key::DeleteWord(false), &tx);
    assert_eq!(a.input, "café ");
}

#[test]
fn line_keys_act_on_the_caret_row() {
    let (tx, _cmds) = channel();
    let mut a = app();
    a.input = "first\nsecond".into();
    a.cursor = "first\nsec".len();
    a.handle_key(Key::Ctrl('k'), &tx);
    assert_eq!(a.input, "first\nsec");
    a.handle_key(Key::Ctrl('a'), &tx);
    assert_eq!(a.cursor, "first\n".len());
    a.cursor = a.input.len();
    a.handle_key(Key::Ctrl('u'), &tx);
    assert_eq!((a.input.as_str(), a.cursor), ("first\n", 6));
}

#[test]
fn question_options_show_descriptions_and_take_several_picks() {
    let (tx, _cmds) = channel();
    let (reply, answers) = channel();
    let mut a = colour_app();
    a.apply(Ev::Question {
        q: "which languages?".into(),
        opts: vec!["Rust (Recommended)".into(), "Go".into(), "Zig".into()],
        descs: vec!["fast, safe, and the one imp is written in".into(), String::new(), "small".into()],
        multiple: true,
        reply,
    });
    let out = strip(&a.frame(54, 24));
    assert!(out.contains("▌     fast, safe, and the one imp is written in") || out.contains("▌     fast, safe, and the one imp is written"), "{out:?}");
    assert!(out.contains("1-3 mark   Enter send"), "{out:?}");
    assert!(out.lines().all(|l| l.trim_end_matches('\r').chars().count() <= 54));
    let muted = a.palette.paint(Role::Thinking, "    small");
    assert!(a.frame(54, 24).contains(&muted), "the description is not muted");
    // Enter with nothing marked keeps the question; marks toggle.
    a.handle_key(Key::Enter, &tx);
    assert!(answers.try_recv().is_err() && a.pending.is_some());
    a.handle_key(Key::Char('1'), &tx);
    a.handle_key(Key::Char('3'), &tx);
    a.handle_key(Key::Char('2'), &tx);
    a.handle_key(Key::Char('2'), &tx);
    assert!(strip(&a.frame(54, 24)).contains("[x] Rust"));
    a.handle_key(Key::Char('9'), &tx);
    assert!(a.pending.is_some(), "an out-of-range key dropped the question");
    a.handle_key(Key::Enter, &tx);
    assert_eq!(answers.try_recv().ok(), Some(Some("Rust (Recommended), Zig".into())));
}

#[test]
fn a_question_waits_for_its_answer() {
    let (tx, rx) = channel();
    let mut hooks = WorkerHooks { tx, waited: Duration::ZERO };
    let asker = std::thread::spawn(move || hooks.ask_choices("which?", &[("a".into(), String::new())], false));
    let Ok(Ev::Question { reply, .. }) = rx.recv_timeout(Duration::from_secs(2)) else { panic!("no question") };
    std::thread::sleep(Duration::from_millis(200));
    reply.send(Some("a".into())).unwrap();
    assert_eq!(asker.join().unwrap().as_deref(), Some("a"));
}

#[test]
fn delete_removes_forward() {
    let (tx, _cmds) = channel();
    let mut a = app();
    a.input = "cat".into();
    a.cursor = 1;
    a.handle_key(Key::Delete, &tx);
    assert_eq!((a.input.as_str(), a.cursor), ("ct", 1));
    a.cursor = 2;
    a.handle_key(Key::Delete, &tx);
    assert_eq!(a.input, "ct", "Delete at the end changed the draft");
    a.input = "né".into();
    a.cursor = 1;
    a.handle_key(Key::Delete, &tx);
    assert_eq!(a.input, "n", "a multi-byte character was split");
}

#[test]
fn the_key_reader_never_reads_while_a_command_holds_the_terminal() {
    use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
    static ATT: AtomicBool = AtomicBool::new(false);
    static PARK: AtomicBool = AtomicBool::new(false);
    let mut fds = [0 as c_int; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0);
    let (tx, rx) = channel();
    let fd = fds[0];
    std::thread::spawn(move || read_keys(fd, tx, &ATT, &PARK));
    let put = |b: &[u8]| unsafe { write(fds[1], b.as_ptr().cast(), b.len()) };
    put(b"a");
    assert_eq!(rx.recv_timeout(Duration::from_secs(2)).ok(), Some(b"a".to_vec()));
    // Attached: the reader parks, and bytes stay for the command.
    ATT.store(true, SeqCst);
    let t = Instant::now();
    while !PARK.load(SeqCst) && t.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(PARK.load(SeqCst), "the reader did not park");
    put(b"for the shell");
    assert!(rx.recv_timeout(Duration::from_millis(300)).is_err(), "it read while attached");
    let mut left = [0u8; 32];
    let n = unsafe { read(fds[0], left.as_mut_ptr().cast(), left.len()) };
    assert_eq!(&left[..n as usize], b"for the shell");
    // Back: it reads again.
    ATT.store(false, SeqCst);
    put(b"b");
    assert_eq!(rx.recv_timeout(Duration::from_secs(2)).ok(), Some(b"b".to_vec()));
}

#[test]
fn alt_and_option_word_keys_decode() {
    let cases: [(&[u8], Key); 9] = [
        (b"\x1bb", Key::Word(Move::Left)),
        (b"\x1bf", Key::Word(Move::Right)),
        (b"\x1bd", Key::DeleteWord(true)),
        (b"\x1b\x7f", Key::DeleteWord(false)),
        (b"\x1b[1;3D", Key::Word(Move::Left)),
        (b"\x1b[1;3C", Key::Word(Move::Right)),
        (b"\x1b[1;4D", Key::SelectWord(Move::Left)),
        (b"\x1b[1;4C", Key::SelectWord(Move::Right)),
        (b"\x1b[3;3~", Key::DeleteWord(true)),
    ];
    for (bytes, want) in cases {
        let mut buf = bytes.to_vec();
        assert_eq!(decode(&mut buf), vec![want], "{bytes:?}");
    }
    // A lone Esc, and Esc followed by another key, stay Esc.
    assert_eq!(decode(&mut b"\x1b".to_vec()), vec![Key::Esc]);
    assert_eq!(decode(&mut b"\x1bx".to_vec()), vec![Key::Esc, Key::Char('x')]);
}

#[test]
fn shift_moves_and_delete_decode() {
    let cases: [(&[u8], Key); 13] = [
        (b"\x1b[1~", Key::Home),
        (b"\x1b[4~", Key::End),
        (b"\x1b[7~", Key::Home),
        (b"\x1b[8~", Key::End),
        (b"\x1b[1;2A", Key::Select(Move::Up)),
        (b"\x1b[1;2B", Key::Select(Move::Down)),
        (b"\x1b[1;2C", Key::Select(Move::Right)),
        (b"\x1b[1;2D", Key::Select(Move::Left)),
        (b"\x1b[1;2H", Key::Select(Move::Home)),
        (b"\x1b[1;2F", Key::Select(Move::End)),
        (b"\x1b[3~", Key::Delete),
        (b"\x1b[3;2~", Key::Delete),
        (b"\x1b[1;5D", Key::Left),
    ];
    for (bytes, want) in cases {
        let mut buf = bytes.to_vec();
        assert_eq!(decode(&mut buf), vec![want], "{bytes:?}");
    }
}

#[test]
fn wrap_respects_width() {
    let lines = wrap("alpha beta gamma delta", 10);
    assert!(lines.iter().all(|l| l.chars().count() <= 10), "{lines:?}");
    assert!(lines.len() > 1);
}

fn strip(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\x1b' {
            while let Some(n) = it.next() {
                if n.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn app() -> App {
    app_at(std::env::temp_dir())
}

/// An app that opens real live sessions, logging under a scratch directory.
fn multi_app(name: &str) -> App {
    let dir = std::env::temp_dir().join(format!("imp-panes-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut a = app_at(dir.clone());
    a.sessions_dir = dir.join("sessions");
    let out = dir.join("out");
    a.fresh = Some(Box::new(move |root: &std::path::Path| {
        let mut cfg = Config::default();
        cfg.color = ColorMode::Never;
        Session::new(
            cfg,
            root.to_path_buf(),
            out.clone(),
            String::new(),
            crate::store::Agent::builtin(),
            Vec::new(),
            std::sync::Arc::new(std::sync::atomic::AtomicU16::new(80)),
        )
    }));
    a
}

#[test]
fn new_opens_a_second_live_session() {
    let mut a = multi_app("second");
    let (tx, _rx) = channel();
    a.push(Role::User, "first question");
    a.status.streaming = true;
    a.run_command("new", &tx);
    assert_eq!((a.panes.len(), a.at), (2, 1));
    assert!(!a.status.streaming, "the new session inherited the turn");
    assert!(!a.entries.iter().any(|e| e.text == "first question"));
    let first = a.panes[0].as_ref().unwrap();
    assert!(first.status.streaming, "the first session stopped");
    assert!(first.entries.iter().any(|e| e.text == "first question"));
    // An event for the first lands there, not in view.
    a.apply_to(0, Ev::Info("from the first".into()));
    assert!(!a.entries.iter().any(|e| e.text == "from the first"));
    assert!(a.panes[0].as_ref().unwrap().entries.iter().any(|e| e.text == "from the first"));
}

#[test]
fn resuming_opens_another_live_session() {
    let mut a = multi_app("resume");
    let (tx, _rx) = channel();
    let log = crate::store::Log::create(&a.sessions_dir, &a.root, "m");
    log.append(&crate::api::Message::user("stored question"));
    a.push(Role::User, "live question");
    a.run_command(&format!("sessions {}", log.id), &tx);
    assert_eq!((a.panes.len(), a.at), (2, 1));
    assert!(a.panes[0].as_ref().unwrap().entries.iter().any(|e| e.text == "live question"));
    // The worker replays the stored turn into the new pane.
    let deadline = Instant::now() + Duration::from_secs(5);
    while !a.entries.iter().any(|e| e.text.contains("stored question")) && Instant::now() < deadline {
        a.poll();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(a.entries.iter().any(|e| e.text.contains("stored question")), "no replay");
}

#[test]
fn enter_with_no_title_match_searches_session_text() {
    let mut a = multi_app("search");
    let (tx, _rx) = channel();
    let log = crate::store::Log::create(&a.sessions_dir, &a.root, "m");
    log.append(&crate::api::Message::user("weather please"));
    log.append(&crate::api::Message::assistant("wttr.in says rain"));
    a.run_command("sessions", &tx);
    for c in "WTTR".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    a.handle_key(Key::Enter, &tx);
    let Some(Pending::Pick(p)) = &a.pending else { panic!("picker closed") };
    assert_eq!(p.title, "search \"WTTR\"");
    assert!(p.filter.is_empty());
    assert_eq!(p.items.len(), 1);
    assert!(p.items[0].label.contains("wttr.in"), "{}", p.items[0].label);
    a.handle_key(Key::Enter, &tx);
    assert_eq!((a.panes.len(), a.at), (2, 1), "the hit did not open");

    a.run_command("sessions", &tx);
    for c in "zzqx".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    a.handle_key(Key::Enter, &tx);
    let Some(Pending::Pick(p)) = &a.pending else { panic!("picker closed") };
    assert!(p.items.is_empty());
    a.handle_key(Key::Enter, &tx);
    assert!(matches!(a.pending, Some(Pending::Pick(_))), "Enter on no hits closed it");
    assert_eq!(a.panes.len(), 2);
}

#[test]
fn working_sessions_spin_in_the_session_picker() {
    let mut a = multi_app("spin");
    let (tx, _rx) = channel();
    a.run_command("new", &tx);
    let id = a.worker.as_ref().unwrap().2.lock().unwrap().clone();
    // A session is listed once its log has a line.
    std::fs::write(a.sessions_dir.join(format!("{id}.jsonl")), "").unwrap();
    a.status.streaming = true;
    a.run_command("sessions", &tx);
    let desc = |a: &App| {
        let Some(Pending::Pick(p)) = &a.pending else { panic!("no picker") };
        p.items.iter().find(|i| i.value == id).expect("row").desc.clone()
    };
    let first = desc(&a);
    assert!(spinner_frames().contains(first.chars().next().unwrap()), "{first}");
    a.last_redraw = Instant::now() - Duration::from_millis(200);
    assert!(a.spin_sessions());
    assert_ne!(desc(&a), first, "the frame did not advance");
    a.status.streaming = false;
    a.last_redraw = Instant::now() - Duration::from_millis(200);
    assert!(a.spin_sessions());
    assert!(desc(&a).ends_with("msgs") && desc(&a).starts_with("  "), "{}", desc(&a));
    assert_eq!(desc(&a).chars().count(), first.chars().count(), "the cell changed width");
    a.last_redraw = Instant::now() - Duration::from_millis(200);
    assert!(!a.spin_sessions(), "an idle picker repainted");
}

#[test]
fn backspace_on_an_empty_draft_removes_the_last_image() {
    // The chat asks only when the draft is empty.
    let (tx, cmds) = channel();
    let mut a = app();
    a.input = "x".into();
    a.cursor = 1;
    a.handle_key(Key::Backspace, &tx);
    assert!(cmds.try_recv().is_err(), "a real delete detached an image");
    a.handle_key(Key::Backspace, &tx);
    assert!(matches!(cmds.try_recv(), Ok(Cmd::Detach)));
    // The worker drops the newest attachment and says which.
    let dir = std::env::temp_dir().join(format!("imp-detach-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let session = Session::new(
        Config::default(),
        dir.clone(),
        dir.join("out"),
        String::new(),
        crate::store::Agent::builtin(),
        Vec::new(),
        std::sync::Arc::new(std::sync::atomic::AtomicU16::new(80)),
    );
    let (tx, rx, _) = spawn_worker(session, dir.join("sessions"));
    let image = |name: &str| crate::api::Image { file: format!("{name}.png"), name: name.into(), mime: "image/png".into(), bytes: 2048 };
    tx.send(Cmd::Attach(vec![image("one"), image("two")])).unwrap();
    tx.send(Cmd::Detach).unwrap();
    tx.send(Cmd::Detach).unwrap();
    tx.send(Cmd::Detach).unwrap();
    let mut said = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    while said.len() < 2 && Instant::now() < deadline {
        if let Ok(Ev::Info(t)) = rx.recv_timeout(Duration::from_millis(100)) {
            said.push(t);
        }
    }
    assert_eq!(said, vec!["removed [image two · 2 KB]", "removed [image one · 2 KB]"]);
    assert!(!matches!(rx.recv_timeout(Duration::from_millis(300)), Ok(Ev::Info(_))), "a third removal spoke");
    let _ = tx.send(Cmd::Quit);
}

#[test]
fn a_due_check_in_runs_a_turn_in_the_worker() {
    let dir = std::env::temp_dir().join(format!("imp-wake-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let mut cfg = Config::default();
    cfg.color = ColorMode::Never;
    let mut session = Session::new(
        cfg,
        dir.clone(),
        dir.join("out"),
        String::new(),
        crate::store::Agent::builtin(),
        Vec::new(),
        std::sync::Arc::new(std::sync::atomic::AtomicU16::new(80)),
    );
    session.transport = Box::new(std::sync::Arc::new(crate::transport::Fake::new(vec![
        crate::transport::Script::Ok(crate::transport::text_reply("the copy is 64% done")),
    ])));
    session.check_in_now();
    let (_tx, rx, _) = spawn_worker(session, dir.join("sessions"));
    let deadline = Instant::now() + Duration::from_secs(10);
    let (mut woke, mut text, mut done) = (None, String::new(), false);
    while !done && Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(Ev::Woke(w)) => woke = Some(w),
            Ok(Ev::Token(t)) => text.push_str(&t),
            Ok(Ev::Done(_)) => done = true,
            _ => {}
        }
    }
    assert_eq!(woke.as_deref(), Some("check-in"));
    assert!(done && text.contains("64% done"), "{text:?}");
}

#[test]
fn a_goal_runs_rounds_in_the_worker_until_met() {
    let dir = std::env::temp_dir().join(format!("imp-goal-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let mut session = Session::new(
        Config::default(),
        dir.clone(),
        dir.join("out"),
        String::new(),
        crate::store::Agent::builtin(),
        Vec::new(),
        std::sync::Arc::new(std::sync::atomic::AtomicU16::new(80)),
    );
    session.transport = Box::new(std::sync::Arc::new(crate::transport::Fake::new(vec![
        crate::transport::Script::Ok(crate::transport::text_reply("fixed one; one left")),
        crate::transport::Script::Ok(crate::transport::text_reply("all green\nGOAL MET 13 tests OK")),
    ])));
    let (tx, rx, _) = spawn_worker(session, dir.join("sessions"));
    tx.send(Cmd::Goal("all tests pass".into())).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let (mut rounds, mut end, mut dones) = (Vec::new(), None, 0);
    while end.is_none() && Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(Ev::GoalRound { round, rounds: n, .. }) => rounds.push((round, n)),
            Ok(Ev::GoalEnd(why)) => end = Some(why),
            Ok(Ev::Done(_)) => dones += 1,
            _ => {}
        }
    }
    assert_eq!(rounds, vec![(1, 5), (2, 5)]);
    assert_eq!(dones, 2);
    assert_eq!(end.as_deref(), Some("goal met"));
}

#[test]
fn goal_command_starts_shows_and_ends() {
    let mut a = app();
    let (tx, rx) = channel();
    a.run_command("goal all tests pass", &tx);
    assert!(matches!(rx.try_recv(), Ok(Cmd::Goal(t)) if t == "all tests pass"));
    assert!(a.entries.iter().any(|e| e.text == "/goal all tests pass"));
    a.apply(Ev::GoalRound { text: "all tests pass".into(), round: 2, rounds: 5 });
    assert!(strip(&a.status_line(100)).contains("goal 2/5"));
    assert!(a.entries.iter().any(|e| e.text.contains("goal round 2/5")));
    a.run_command("goal", &tx);
    assert!(a.entries.last().unwrap().text.contains("all tests pass · round 2/5"));
    a.run_command("goal off", &tx);
    assert!(matches!(rx.try_recv(), Ok(Cmd::GoalOff)));
    a.apply(Ev::GoalEnd("goal cleared".into()));
    assert!(!strip(&a.status_line(100)).contains("goal "));
}

#[test]
fn a_woken_session_out_of_view_joins_the_queue() {
    let mut a = multi_app("woke");
    let (tx, _rx) = channel();
    a.run_command("new", &tx);
    assert_eq!((a.at, a.queue().len()), (1, 0));
    a.apply_to(0, Ev::Woke("check-in: job 3".into()));
    assert!(a.panes[0].as_ref().unwrap().status.streaming, "the woken session is not working");
    a.apply_to(0, Ev::Token("64% done".into()));
    a.apply_to(0, Ev::Done(None));
    assert_eq!(a.queue(), vec![0], "the woken session is not waiting");
    assert!(a.panes[0].as_ref().unwrap().entries.iter().any(|e| e.text == "⏰ check-in: job 3"));
    assert_eq!(wake_summary("[Check-in from imp]\nJob 3 ended with exit 0. job 3 x"), "job ended: job 3");
    assert_eq!(
        wake_summary("Job 3 ended with exit 0.\n\nThe check-in you asked for 30s ago:\njob 3  exit 0"),
        "job ended, check-in: job 3"
    );
}

#[test]
fn ctrl_p_opens_the_palette_with_commands_and_keys() {
    let mut a = app();
    let (tx, _cmds) = channel();
    a.handle_key(Key::Ctrl('p'), &tx);
    let picker = open_picker(&a);
    assert_eq!(picker.kind, PickKind::Palette);
    assert_eq!(picker.title, "palette");
    let values: Vec<&str> = picker.items.iter().map(|i| i.value.as_str()).collect();
    assert!(values.contains(&"cmd:new"), "no command rows");
    assert!(values.contains(&"key:expand"), "no key rows");
    let expand = picker.items.iter().find(|i| i.value == "key:expand").unwrap();
    assert_eq!(expand.desc, "Ctrl+E", "the shortcut is not shown");
    assert!(strip(&a.frame(90, 20)).contains("palette"), "the picker title is not drawn");
}

#[test]
fn the_palette_runs_a_command() {
    let mut a = app();
    let (tx, _cmds) = channel();
    a.handle_key(Key::Ctrl('p'), &tx);
    for c in "keys".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    if let Some(Pending::Pick(p)) = &mut a.pending {
        p.index = 0;
    }
    a.handle_key(Key::Enter, &tx);
    assert_eq!(open_picker(&a).kind, PickKind::Keys, "the command did not run");
}

#[test]
fn the_palette_replays_a_key() {
    let mut a = app();
    let (tx, _cmds) = channel();
    a.push(Role::User, "something to clear");
    a.handle_key(Key::Ctrl('p'), &tx);
    for c in "clear".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    assert!(
        open_picker(&a).items.iter().any(|i| i.value == "key:clear"),
        "the clear row"
    );
    if let Some(Pending::Pick(p)) = &mut a.pending {
        p.index = 0;
    }
    a.handle_key(Key::Enter, &tx);
    assert!(a.entries.is_empty(), "the key did not replay");
    assert!(a.pending.is_none());
}

#[test]
fn ctrl_d_keeps_a_draft() {
    let mut a = app();
    let (tx, _cmds) = channel();
    a.input = "half a thought".into();
    a.cursor = a.input.len();
    a.handle_key(Key::Ctrl('d'), &tx);
    assert!(!a.quit, "Ctrl+D quit with a draft");
    assert_eq!(a.input, "half a thought");
    a.input.clear();
    a.cursor = 0;
    a.handle_key(Key::Ctrl('d'), &tx);
    assert!(a.quit, "Ctrl+D did not quit from an empty draft");
}

#[test]
fn every_palette_action_has_a_handler() {
    let a = app();
    let actions: Vec<String> = a.palette_items().into_iter().filter_map(|i| i.value.strip_prefix("key:").map(String::from)).collect();
    assert!(!actions.iter().any(|v| v == "thinking"), "/think is already a palette row");
    assert!(!actions.iter().any(|v| v == "keys"), "/keys is already a palette row");
}

#[test]
fn the_palette_shows_command_keys_and_more_actions() {
    let mut a = app();
    let (tx, _cmds) = channel();
    let items = a.palette_items();
    let desc = |label: &str| items.iter().find(|i| i.label == label).map(|i| i.desc.clone()).unwrap_or_default();
    assert!(desc("/keys").ends_with(" · ?"), "{}", desc("/keys"));
    assert!(desc("/next").ends_with(" · Ctrl+O"), "{}", desc("/next"));
    assert!(!desc("/mode").contains('·'), "/mode does not do what Shift+Tab does");
    assert_eq!(desc("steer now"), "Ctrl+S");
    assert_eq!(desc("paste"), "Ctrl+V");
    let before = a.entries.len();
    a.palette_action("steer", &tx);
    assert_eq!(a.entries.len(), before, "steer now with an empty draft did something");
    a.palette_action("next-session", &tx);
    assert_eq!(a.entries.last().unwrap().text, "one session");
}

#[test]
fn retry_drops_the_reply_and_sends_again() {
    let mut a = app();
    let (tx, rx) = channel();
    a.handle_key(Key::Ctrl('r'), &tx);
    assert_eq!(a.entries.last().unwrap().text, "nothing to retry");
    a.entries.clear();
    a.push(Role::User, "first");
    a.push(Role::Assistant, "one");
    a.push(Role::User, "second");
    a.apply(Ev::ToolStart { name: "bash".into(), summary: "run `ls`".into(), args: "{}".into() });
    a.apply(Ev::ToolEnd { ok: true, preview: "exit 0".into(), output: String::new(), detail: String::new(), ms: 1 });
    a.push(Role::Assistant, "a bad answer");
    a.status.streaming = true;
    a.run_command("retry", &tx);
    assert_eq!(a.entries.last().unwrap().text, "a turn is running");
    assert!(rx.try_recv().is_err(), "retried mid-turn");
    a.status.streaming = false;
    a.handle_key(Key::Ctrl('r'), &tx);
    let texts: Vec<&str> = a.entries.iter().map(|e| e.text.as_str()).collect();
    assert_eq!(texts, vec!["first", "one", "second"], "the old reply stayed");
    assert!(a.done.is_empty() && a.tool.is_none(), "a tool row of the old reply stayed");
    assert!(a.status.streaming, "the retry did not start a turn");
    assert!(matches!(rx.try_recv(), Ok(Cmd::Retry(t)) if t == "second"));
}

#[test]
fn retry_skips_shell_and_goal_rows() {
    let mut a = app();
    let (tx, rx) = channel();
    a.push(Role::User, "fix it (steered)");
    a.push(Role::Assistant, "done");
    a.push(Role::User, "/goal tests pass");
    a.push(Role::Assistant, "round one");
    a.push(Role::User, "!ls");
    a.push(Role::Info, "Cargo.toml");
    a.handle_key(Key::Ctrl('r'), &tx);
    assert!(matches!(rx.try_recv(), Ok(Cmd::Retry(t)) if t == "fix it"), "retried a row the agent never saw");
    let texts: Vec<&str> = a.entries.iter().map(|e| e.text.as_str()).collect();
    assert_eq!(texts, vec!["fix it (steered)"]);
}

#[test]
fn ctrl_t_toggles_thinking() {
    let mut a = app();
    let (tx, rx) = channel();
    let was = a.cfg.thinking;
    a.handle_key(Key::Ctrl('t'), &tx);
    assert_ne!(a.cfg.thinking, was, "Ctrl+T did not toggle thinking");
    assert!(matches!(rx.try_recv(), Ok(Cmd::SetThinking(on)) if on != was));
    let think = a.palette_items().into_iter().find(|i| i.label == "/think").unwrap();
    assert!(think.desc.ends_with(" · Ctrl+T"), "{}", think.desc);
    let retry = a.palette_items().into_iter().find(|i| i.label == "/retry").unwrap();
    assert!(retry.desc.ends_with(" · Ctrl+R"), "{}", retry.desc);
}

/// The keys a keys card row names. A row missing here fails the test.
fn card_keys(label: &str) -> Option<Vec<Key>> {
    use Move::*;
    Some(match label {
        "Enter" => vec![Key::Enter],
        "Shift+Enter, Alt+Enter, Ctrl+J" => vec![Key::Newline],
        "Esc" => vec![Key::Esc],
        "Tab" => vec![Key::Tab],
        "Shift+Tab" => vec![Key::BackTab],
        "Ctrl+B" => vec![Key::Ctrl('b')],
        "Right" => vec![Key::Right],
        "Up, Down" => vec![Key::Up, Key::Down],
        "Shift+arrows, Shift+Home, Shift+End" => [Up, Down, Left, Right, Home, End].map(Key::Select).to_vec(),
        "Delete" => vec![Key::Delete],
        "Option+Left, Option+Right" => vec![Key::Word(Left), Key::Word(Right)],
        "Option+Backspace, Ctrl+W" => vec![Key::DeleteWord(false), Key::Ctrl('w')],
        "Option+Delete, Alt+D" => vec![Key::DeleteWord(true)],
        "Shift+Option+Left, Shift+Option+Right" => vec![Key::SelectWord(Left), Key::SelectWord(Right)],
        "Ctrl+U, Ctrl+K" => vec![Key::Ctrl('u'), Key::Ctrl('k')],
        "?" => vec![Key::Char('?')],
        "PgUp, PgDn" => vec![Key::PageUp, Key::PageDown],
        "PgUp" => vec![Key::PageUp],
        "PgDn" => vec![Key::PageDown],
        "Picker" => vec![Key::Char('z'), Key::Enter, Key::Esc],
        l => match l.strip_prefix("Ctrl+") {
            Some(c) if c.len() == 1 => vec![Key::Ctrl(c.to_ascii_lowercase().chars().next()?)],
            _ => return None,
        },
    })
}

/// An app in the state where `key` acts; `picker` opens the palette first.
fn key_state(key: &Key, picker: bool) -> App {
    let mut a = if *key == Key::Ctrl('n') { multi_app("card-keys") } else { app() };
    let (tx, _rx) = channel();
    if picker {
        a.handle_key(Key::Ctrl('p'), &tx);
        return a;
    }
    let draft = |a: &mut App, text: &str, at_end: bool| {
        a.input = text.into();
        a.cursor = if at_end { text.len() } else { 0 };
    };
    match key {
        // Ctrl+S on an empty draft does nothing by design, so it gets a draft.
        // Ctrl+C now stops a running turn; it does nothing to an idle draft.
        Key::Ctrl('c') => {
            draft(&mut a, "hello", true);
            a.status.streaming = true;
        }
        Key::Enter | Key::Newline | Key::Ctrl('s') => draft(&mut a, "hello", true),
        Key::Delete | Key::DeleteWord(true) | Key::Ctrl('k') => draft(&mut a, "hello world", false),
        Key::Select(Move::Down | Move::Right | Move::End) | Key::Word(Move::Right) | Key::SelectWord(Move::Right) => {
            draft(&mut a, "hello world\nsecond", false)
        }
        Key::Select(_) | Key::Word(_) | Key::SelectWord(_) | Key::DeleteWord(false) | Key::Ctrl('w' | 'a' | 'u') => {
            draft(&mut a, "hello world\nsecond", true)
        }
        Key::Esc => a.status.streaming = true,
        // Tab completes a command, so it needs a command draft to act on.
        Key::Tab => draft(&mut a, "/mo", true),
        Key::BackTab => a.agents = vec![("build".into(), String::new()), ("review".into(), String::new())],
        Key::Right => a.status.suggestion = Some("run the tests".into()),
        Key::Up | Key::Down => {
            a.history = vec!["older".into(), "newer".into()];
            if *key == Key::Down {
                a.handle_key(Key::Up, &tx);
            }
        }
        Key::Ctrl('x') => {
            a.queued = vec!["first".into(), "second".into()];
            a.status.streaming = true;
            a.handle_key(Key::Up, &tx);
        }
        Key::Ctrl('l') => a.push(Role::User, "something to clear"),
        Key::Ctrl('e') => {
            a.apply(Ev::ToolStart { name: "bash".into(), summary: "run `ls`".into(), args: "{\"command\":\"ls\"}".into() });
            a.apply(Ev::ToolEnd { ok: true, preview: "exit 0".into(), output: "a\nb".into(), detail: String::new(), ms: 1 });
        }
        Key::Ctrl('v') => a.clip = "pasted".into(),
        Key::Ctrl('g') => a.cfg.mouse = true,
        Key::Ctrl('n') => a.run_command("new", &tx),
        Key::PageUp | Key::PageDown => {
            for i in 0..200 {
                a.push(Role::Info, format!("line {i}"));
            }
            if *key == Key::PageDown {
                a.handle_key(Key::PageUp, &tx);
            }
        }
        _ => {}
    }
    a
}

/// What a key can change: the drawn screen and the state it does not draw.
fn fingerprint(a: &mut App) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        a.frame(80, 20),
        a.input,
        a.cursor,
        a.quit,
        a.at,
        a.released,
        a.agent,
        a.cancel.load(Ordering::Relaxed)
    )
}

#[test]
fn every_listed_key_acts() {
    for (label, _) in crate::help::KEYS {
        let keys = card_keys(label).unwrap_or_else(|| panic!("keys card row {label:?} has no key mapping"));
        for key in keys {
            let mut a = key_state(&key, *label == "Picker");
            let before = fingerprint(&mut a);
            let (tx, _rx) = channel();
            a.handle_key(key.clone(), &tx);
            assert_ne!(before, fingerprint(&mut a), "{label}: {key:?} did nothing");
        }
    }
    for (label, key, action) in PALETTE_ACTIONS {
        let keys = card_keys(key).unwrap_or_else(|| panic!("palette row {label:?} names {key:?}, not a key"));
        let mut a = key_state(&keys[0], false);
        let before = fingerprint(&mut a);
        let (tx, _rx) = channel();
        a.palette_action(action, &tx);
        assert_ne!(before, fingerprint(&mut a), "palette {label} did nothing");
    }
}

#[test]
fn ctrl_n_and_p_switch_sessions_and_keep_drafts() {
    let mut a = multi_app("switch");
    let (tx, _rx) = channel();
    a.handle_key(Key::Ctrl('n'), &tx);
    assert_eq!(a.panes.len(), 1, "one session must not switch");
    assert!(a.status_line(80).starts_with(&a.cfg.model), "{}", a.status_line(80));
    a.push(Role::User, "fix the parser");
    a.input = "draft one".into();
    a.cursor = a.input.len();
    a.run_command("new", &tx);
    a.input = "draft two".into();
    assert!(strip(&a.title_bar(80)).starts_with(" 2/2 imp "), "{}", strip(&a.title_bar(80)));
    assert!(a.status_line(80).starts_with(&a.cfg.model), "the position stayed in the status line");
    a.handle_key(Key::Ctrl('n'), &tx);
    assert_eq!((a.at, a.input.as_str()), (0, "draft one"));
    assert!(strip(&a.title_bar(80)).starts_with(" 1/2 fix the parser "), "{}", strip(&a.title_bar(80)));
    a.handle_key(Key::Ctrl('p'), &tx);
    let picker = open_picker(&a);
    assert_eq!(picker.kind, PickKind::Palette, "Ctrl+P left the palette");
    assert_eq!((a.at, a.input.as_str()), (0, "draft one"), "Ctrl+P switched session");
}

/// Three live sessions: 0 waits for approval, 1 has replied, 2 is in view.
fn inbox_app(name: &str) -> (App, Receiver<Answer>) {
    let mut a = multi_app(name);
    let (tx, _rx) = channel();
    a.push(Role::User, "run the tests");
    a.status.streaming = true;
    a.run_command("new", &tx);
    a.push(Role::User, "summarise the log");
    a.run_command("new", &tx);
    a.push(Role::User, "in view");
    let (reply, answers) = channel();
    a.apply_to(0, Ev::Confirm { what: "run `npm test`".into(), reply, always: true });
    a.apply_to(1, Ev::Token("All done.".into()));
    a.apply_to(1, Ev::Done(None));
    (a, answers)
}

#[test]
fn ctrl_o_walks_the_queue_oldest_first() {
    let (mut a, answers) = inbox_app("queue");
    let (tx, _rx) = channel();
    assert_eq!(a.queue(), vec![0, 1], "the approval came first");
    a.handle_key(Key::Ctrl('o'), &tx);
    assert_eq!(a.at, 0);
    assert!(matches!(a.pending, Some(Pending::Confirm { .. })));
    // Moving on leaves the approval open and sends it to the back.
    a.handle_key(Key::Ctrl('o'), &tx);
    assert_eq!(a.at, 1, "the reply was skipped");
    assert!(answers.try_recv().is_err(), "moving on answered the approval");
    assert_eq!(a.queue(), vec![0]);
    // The reply in view counts as read once the user moves on.
    a.handle_key(Key::Ctrl('o'), &tx);
    assert_eq!(a.at, 0);
    assert!(!a.panes[1].as_ref().unwrap().waits(), "the read reply still waits");
    a.handle_key(Key::Char('y'), &tx);
    assert_eq!(answers.try_recv().ok(), Some(Answer::Yes));
    a.handle_key(Key::Ctrl('o'), &tx);
    assert_eq!(a.at, 0, "moved with nothing waiting");
    assert_eq!(a.entries.last().unwrap().text, "nothing waiting");
    a.run_command("next", &tx);
    assert_eq!(a.entries.last().unwrap().text, "nothing waiting", "/next differs from Ctrl+O");
}

#[test]
fn handing_off_moves_to_the_next_waiting_session() {
    let (mut a, _answers) = inbox_app("handoff");
    let (tx, _rx) = channel();
    // Typing never switches, even with sessions waiting.
    a.handle_key(Key::Char('h'), &tx);
    a.handle_key(Key::Char('i'), &tx);
    assert_eq!(a.at, 2, "typing switched sessions");
    a.handle_key(Key::Enter, &tx);
    assert_eq!(a.at, 0, "sending did not move on");
    assert!(a.entries.last().unwrap().text.starts_with("→ 1/3 run the tests · needs you"), "{}", a.entries.last().unwrap().text);
    // Answering hands off too: the reply is next.
    a.handle_key(Key::Char('y'), &tx);
    assert_eq!(a.at, 1);
    assert_eq!(a.entries.last().unwrap().text, "→ 2/3 summarise the log · replied");

    let (mut off, _answers) = inbox_app("handoff-off");
    off.cfg.auto_next = false;
    off.handle_key(Key::Char('x'), &tx);
    off.handle_key(Key::Enter, &tx);
    assert_eq!(off.at, 2, "auto_next=off still moved");
}

#[test]
fn a_prompt_in_view_survives_ctrl_o() {
    let mut a = multi_app("survive");
    let (tx, _rx) = channel();
    let (reply, answers) = channel();
    a.apply(Ev::Confirm { what: "run `make`".into(), reply, always: false });
    a.handle_key(Key::Ctrl('o'), &tx);
    assert!(matches!(a.pending, Some(Pending::Confirm { .. })), "the prompt was lost");
    a.handle_key(Key::Char('y'), &tx);
    assert_eq!(answers.try_recv().ok(), Some(Answer::Yes));
}

#[test]
fn sessions_out_of_view_ask_for_attention() {
    let (mut a, answers) = inbox_app("attention");
    // The last background event raised a toast, which takes the status line for
    // three seconds; the waiting count shows once it has gone.
    a.status.toast = None;
    assert!(a.status_line(120).contains(" · 2 waiting"), "{}", a.status_line(120));
    assert!(a.bell_pending, "no bell for a session out of view");
    assert!(!a.panes[0].as_ref().unwrap().bell_pending, "the bell would ring again on switching");
    // Brought into view, the prompt is still open.
    a.show(0);
    assert!(matches!(a.pending, Some(Pending::Confirm { .. })));
    assert!(answers.try_recv().is_err(), "a prompt was answered by switching");
    assert!(a.status_line(120).contains(" · 1 waiting"), "{}", a.status_line(120));
}

#[test]
fn question_mark_opens_the_keys_card_only_from_an_empty_composer() {
    let mut a = app();
    let (tx, _rx) = channel();
    a.handle_key(Key::Char('?'), &tx);
    let Some(Pending::Pick(p)) = &a.pending else { panic!("no keys card") };
    assert_eq!(p.kind, PickKind::Keys);
    assert!(p.items.iter().any(|i| i.label == "Ctrl+O"));
    assert_eq!(p.footer(), "? or Esc close");
    assert!(a.input.is_empty(), "the ? reached the composer");
    a.handle_key(Key::Char('?'), &tx);
    assert!(a.pending.is_none(), "? did not close the card");
    for c in "why".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    a.handle_key(Key::Char('?'), &tx);
    assert_eq!(a.input, "why?");
    assert!(a.pending.is_none());
}

#[test]
fn modes_stay_per_session() {
    let mut a = multi_app("modes");
    let (tx, _rx) = channel();
    a.run_command("new", &tx);
    a.run_command("mode yolo", &tx);
    assert_eq!(a.cfg.live_permissions(), crate::config::Permissions::Yolo);
    assert_eq!(
        a.panes[0].as_ref().unwrap().cfg.live_permissions(),
        crate::config::Permissions::Normal,
        "/mode leaked into another session"
    );
}

#[test]
fn a_fifth_session_closes_the_oldest_idle_one() {
    let mut a = multi_app("room");
    let (tx, _rx) = channel();
    for _ in 0..3 {
        a.status.streaming = true;
        a.run_command("new", &tx);
    }
    assert_eq!(a.panes.len(), 4);
    // Session 0 is busy, 1 and 2 busy, the one in view is idle.
    a.run_command("new", &tx);
    assert_eq!(a.panes.len(), 4, "no room was made");
    assert!(a.panes.iter().flatten().all(|p| p.status.streaming), "an idle session survived");

    a.status.streaming = true;
    a.run_command("new", &tx);
    assert_eq!(a.panes.len(), 4);
    let last = a.entries.last().unwrap();
    assert_eq!(last.role, Role::Error);
    assert!(last.text.starts_with("all four sessions are busy"), "{}", last.text);
}

fn app_at(root: std::path::PathBuf) -> App {
    let mut cfg = Config::default();
    cfg.color = ColorMode::Never;
    App::new(
        cfg,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        std::sync::Arc::new(std::sync::atomic::AtomicU16::new(80)),
        root,
        "default".into(),
    )
}

/// A unique empty directory for a browser fixture.
fn scratch(tag: &str) -> std::path::PathBuf {
    // One directory per call: tests sharing a tag run in parallel.
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("imp-picker-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.canonicalize().unwrap()
}

fn colour_app() -> App {
    let mut cfg = Config::default();
    cfg.color = ColorMode::Always;
    let mut a = App::new(
        cfg,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        std::sync::Arc::new(std::sync::atomic::AtomicU16::new(80)),
        std::env::temp_dir(),
        "default".into(),
    );
    // Pin the palette: the saved theme on this machine must not matter.
    a.palette = Palette {
        on: true,
        allowed: true,
        truecolor: true,
        theme: crate::theme::by_name("default").unwrap(),
    };
    a
}

#[test]
fn esc_stops_a_running_turn() {
    let mut a = app();
    let flag = a.cancel.clone();
    let (tx, _rx) = channel();
    a.status.streaming = true;
    a.handle_key(Key::Esc, &tx);
    assert!(
        flag.load(Ordering::Relaxed),
        "Esc did not ask the running turn to stop"
    );
}

#[test]
fn up_moves_between_rows_before_history() {
    let mut a = app();
    a.history = vec!["old".to_string()];
    a.input = "aa\nbbb".into();
    a.cursor = a.input.len();
    let (tx, _rx) = channel();
    a.handle_key(Key::Up, &tx);
    assert_eq!(a.input, "aa\nbbb", "Up left the composer");
    assert_eq!(a.cursor, 2, "Up did not land on the row above");
    assert!(
        a.hist_pos.is_none(),
        "Up recalled history from the last row"
    );
}

#[test]
fn a_long_prompt_keeps_its_answer_hint() {
    let mut a = app();
    let long = format!("run `{}`", "cat openspec/changes/thing.md; ".repeat(8));
    let (reply, _rx) = channel();
    a.apply(Ev::Confirm { what: long, reply, always: false });
    let out = strip(&a.frame(80, 12));
    let row = out
        .lines()
        .find(|l| l.contains("[y] yes  [n] no"))
        .unwrap_or_else(|| panic!("no answer hint in {out:?}"));
    assert!(row.contains('s'), "no seconds left: {row:?}");
    assert!(
        strip(row).chars().filter(|c| *c != '\u{7}').count() <= 80,
        "the prompt row is wider than the terminal"
    );
}

#[test]
fn esc_refuses_a_prompt_without_quitting() {
    let mut a = app();
    let (reply, rx) = channel();
    a.apply(Ev::Confirm {
        what: "run `ls`".into(),
        reply,
        always: false,
    });
    a.status.streaming = true;
    let (tx, _cmds) = channel();
    a.handle_key(Key::Esc, &tx);
    assert!(a.pending.is_none(), "the prompt stayed open");
    assert_eq!(
        rx.try_recv().ok(),
        Some(Answer::No),
        "the prompt was not refused"
    );
    assert!(!a.quit, "Esc quit the whole interface");
}

#[test]
fn a_new_prompt_rings_once() {
    let mut a = app();
    let (reply, _rx) = channel();
    a.apply(Ev::Confirm {
        what: "run `ls`".into(),
        reply,
        always: false,
    });
    assert!(a.frame(80, 12).contains('\u{7}'), "no bell on a new prompt");
    a.bell_pending = false;
    assert!(
        !a.frame(80, 12).contains('\u{7}'),
        "the same prompt rings twice"
    );
}

#[test]
fn a_labelled_call_names_its_command_when_confirming() {
    let mut a = app();
    a.apply(Ev::ToolStart {
        name: "bash".into(),
        summary: "count files".into(),
        args: "{}".into(),
    });
    let (reply, _rx) = channel();
    a.apply(Ev::Confirm { what: "run `find . | wc -l`".into(), reply, always: false });
    let frame = strip(&a.frame(80, 20));
    assert!(frame.contains("  $ find . | wc -l\r\n"), "{frame:?}");
    assert!(frame.contains("▌ run it?   [y] yes  [n] no"), "{frame:?}");
}

#[test]
fn the_confirm_row_has_no_countdown_and_enter_approves() {
    let mut a = app();
    let (tx, _cmds) = channel();
    let (reply, answers) = channel();
    a.apply(Ev::Confirm { what: "run `npm test`".into(), reply, always: true });
    let frame = strip(&a.frame(80, 12));
    let row = frame.lines().find(|l| l.contains("[a] always")).expect("chips");
    assert_eq!(row.trim_end().trim_start_matches('\u{7}'), "▌ run it?   [y] yes  [n] no  [s] session  [a] always");
    a.handle_key(Key::Enter, &tx);
    assert_eq!(answers.try_recv().ok(), Some(Answer::Yes));
}

/// Click the middle of the first target that matches.
fn click(a: &mut App, tx: &Sender<Cmd>, want: impl Fn(&Target) -> bool) {
    let t = a.targets.iter().find(|t| want(&t.3)).cloned().expect("no such target");
    a.handle_key(Key::Click(((t.1 + t.2) / 2) as u16, t.0 as u16), tx);
}

#[test]
fn a_click_on_a_chip_answers_like_its_key() {
    let mut a = app();
    let (tx, _cmds) = channel();
    let (reply, rx) = channel();
    a.apply(Ev::Confirm { what: "run `npm test`".into(), reply, always: true });
    a.frame(80, 12);
    click(&mut a, &tx, |t| matches!(t, Target::Key(Key::Char('n'))));
    assert_eq!(rx.try_recv().ok(), Some(Answer::No));
}

#[test]
fn a_click_on_an_option_answers_the_question() {
    let mut a = app();
    let (tx, _cmds) = channel();
    let (reply, rx) = channel();
    a.apply(Ev::Question { q: "which?".into(), opts: vec!["red".into(), "blue".into()], descs: Vec::new(), multiple: false, reply });
    a.frame(80, 16);
    click(&mut a, &tx, |t| matches!(t, Target::Key(Key::Char('2'))));
    assert_eq!(rx.try_recv().ok(), Some(Some("blue".to_string())));
}

#[test]
fn a_click_on_a_picker_row_chooses_it_and_a_hint_runs_its_key() {
    let mut a = app();
    let (tx, _cmds) = channel();
    a.run_command("mode", &tx);
    a.frame(100, 30);
    click(&mut a, &tx, |t| matches!(t, Target::Row(3)));
    assert_eq!(a.cfg.live_permissions(), crate::config::Permissions::Yolo);

    a.run_command("mode", &tx);
    a.frame(100, 30);
    click(&mut a, &tx, |t| matches!(t, Target::Key(Key::Esc)));
    assert!(a.pending.is_none(), "Esc close did not close the picker");
}

#[test]
fn always_takes_two_clicks_but_one_key() {
    let mut a = app();
    let (tx, _cmds) = channel();
    let (reply, rx) = channel();
    a.apply(Ev::Confirm { what: "run `npm test`".into(), reply, always: true });
    a.frame(80, 12);
    let always = |t: &Target| matches!(t, Target::Key(Key::Char('a')));
    click(&mut a, &tx, always);
    assert!(rx.try_recv().is_err(), "one click applied always");
    assert!(a.armed);
    a.frame(80, 12);
    click(&mut a, &tx, always);
    assert_eq!(rx.try_recv().ok(), Some(Answer::Always));

    // Anything else in between clears the mark.
    let (reply, rx) = channel();
    a.apply(Ev::Confirm { what: "run `npm test`".into(), reply, always: true });
    a.frame(80, 12);
    click(&mut a, &tx, always);
    a.handle_key(Key::Click(1, 1), &tx);
    assert!(!a.armed);
    a.frame(80, 12);
    click(&mut a, &tx, always);
    assert!(rx.try_recv().is_err(), "the mark survived another click");

    let (reply, rx) = channel();
    a.apply(Ev::Confirm { what: "run `npm test`".into(), reply, always: true });
    a.handle_key(Key::Char('a'), &tx);
    assert_eq!(rx.try_recv().ok(), Some(Answer::Always));
}

#[test]
fn copy_labels_copy_a_reply_or_its_code() {
    let mut a = app();
    let (tx, _cmds) = channel();
    let reply = "Run this:\n```sh\ncargo test\n```\nthen push.";
    a.push(Role::Assistant, reply);
    let frame = strip(&a.frame(40, 12));
    for line in frame.split("\r\n") {
        assert!(line.chars().count() <= 40, "too wide: {line:?}");
    }
    assert_eq!(frame.matches(COPY).count(), 2, "{frame}");
    click(&mut a, &tx, |t| matches!(t, Target::Copy(_, Some(0))));
    assert_eq!(a.clip, "cargo test");
    a.frame(40, 12);
    click(&mut a, &tx, |t| matches!(t, Target::Copy(_, None)));
    assert_eq!(a.clip, reply);
    assert!(a.status.toast.as_ref().is_some_and(|(t, _)| *t == format!("copied {} characters", reply.len())));
}

#[test]
fn ctrl_v_pastes_the_copy_and_osc52_carries_it() {
    let mut a = app();
    let (tx, _cmds) = channel();
    a.handle_key(Key::Ctrl('v'), &tx);
    assert_eq!(a.input, "", "an empty buffer changed the composer");
    a.handle_key(Key::Char('>'), &tx);
    a.copy("a\nb".into());
    assert_eq!(a.osc52.as_deref(), Some("\x1b]52;c;YQpi\x07"));
    a.handle_key(Key::Ctrl('v'), &tx);
    assert_eq!(a.input, ">a\nb");
}

#[test]
fn copy_command_takes_the_last_reply_or_code_block() {
    let mut a = app();
    let (tx, cmds) = channel();
    a.run_command("copy", &tx);
    assert_eq!(a.entries.last().unwrap().text, "nothing to copy");
    a.push(Role::Assistant, "first\n```\nold\n```");
    a.push(Role::Assistant, "no code here");
    a.run_command("copy", &tx);
    assert_eq!(a.clip, "no code here");
    a.run_command("copy code", &tx);
    assert_eq!(a.clip, "old", "the newest block is in an earlier reply");
    assert!(cmds.try_recv().is_err(), "a request was sent");
}

#[test]
fn ctrl_g_releases_the_mouse_until_the_next_key() {
    let mut a = app();
    let (tx, _cmds) = channel();
    a.handle_key(Key::Ctrl('g'), &tx);
    assert!(a.released);
    assert_eq!(a.mouse_seq, Some(MOUSE_OFF));
    assert!(strip(&a.status_line(80)).contains("mouse released"));
    a.mouse_seq = None;
    a.handle_key(Key::Char('h'), &tx);
    assert!(!a.released);
    assert_eq!(a.mouse_seq, Some(MOUSE_ON));
    assert_eq!(a.input, "h", "the key that took the mouse back was lost");

    let mut b = app();
    b.cfg.mouse = false;
    b.handle_key(Key::Ctrl('g'), &tx);
    assert!(!b.released && b.mouse_seq.is_none());
}

#[test]
fn no_copy_labels_with_the_mouse_off() {
    let mut a = app();
    a.cfg.mouse = false;
    a.push(Role::Assistant, "hello\n```\nx\n```");
    assert!(!a.frame(40, 12).contains(COPY));
    assert!(!a.targets.iter().any(|t| matches!(t.3, Target::Copy(..))));
}

#[test]
fn a_click_elsewhere_does_nothing() {
    let mut a = app();
    let (tx, _cmds) = channel();
    let (reply, rx) = channel();
    a.apply(Ev::Confirm { what: "run `npm test`".into(), reply, always: false });
    a.frame(80, 12);
    a.handle_key(Key::Click(1, 1), &tx);
    assert!(rx.try_recv().is_err());
    assert!(a.pending.is_some());
}

#[test]
fn always_is_answered_only_where_offered() {
    let mut a = app();
    let (tx, _cmds) = channel();
    let (reply, rx) = channel();
    a.apply(Ev::Confirm { what: "run `npm test`".into(), reply, always: true });
    assert!(strip(&a.frame(80, 12)).contains("[y] yes  [n] no  [s] session  [a] always"));
    a.handle_key(Key::Char('a'), &tx);
    assert_eq!(rx.try_recv().ok(), Some(Answer::Always));

    let (reply, rx) = channel();
    a.apply(Ev::Confirm { what: "run `npm test`".into(), reply, always: true });
    a.handle_key(Key::Char('s'), &tx);
    assert_eq!(rx.try_recv().ok(), Some(Answer::Session));

    let (reply, rx) = channel();
    a.apply(Ev::Confirm { what: "run `a && b`".into(), reply, always: false });
    let frame = strip(&a.frame(80, 12));
    assert!(frame.contains("[y] yes  [n] no") && !frame.contains("[s]"), "{frame}");
    a.handle_key(Key::Char('s'), &tx);
    a.handle_key(Key::Char('a'), &tx);
    assert!(rx.try_recv().is_err(), "a answered a prompt that did not offer it");
    assert!(matches!(a.pending, Some(Pending::Confirm { .. })));
}

#[test]
fn an_unanswered_prompt_waits() {
    let mut a = app();
    let (reply, answers) = channel();
    a.status.streaming = true;
    a.apply(Ev::Confirm { what: "run `make`".into(), reply, always: false });
    a.frame(80, 12);
    assert!(a.pending.is_some() && a.status.streaming, "the prompt gave up");
    assert!(answers.try_recv().is_err());
    assert!(!a.cancel.load(Ordering::Relaxed), "the turn was cancelled");
}


#[test]
fn up_recalls_history_from_the_first_row() {
    let mut a = app();
    a.history = vec!["old".to_string()];
    a.input = "aa".into();
    a.cursor = 2;
    let (tx, _rx) = channel();
    a.handle_key(Key::Up, &tx);
    assert_eq!(a.input, "old");
}

#[test]
fn down_moves_between_rows() {
    let mut a = app();
    a.input = "aa\nbbb".into();
    a.cursor = 0;
    let (tx, _rx) = channel();
    a.handle_key(Key::Down, &tx);
    assert_eq!(a.cursor, 3, "Down did not land on the row below");
    assert_eq!(a.input, "aa\nbbb");
}

#[test]
fn a_rule_sets_the_composer_apart_and_turns_are_spaced() {
    let mut a = app();
    a.push(Role::User, "first");
    a.push(Role::Assistant, "reply");
    a.push(Role::User, "second");
    let frame = strip(&a.frame(80, 24));
    let rows: Vec<&str> = frame.split("\r\n").collect();
    let rule = rows.iter().position(|r| r.starts_with("────")).expect("no rule");
    assert!(rows[rule + 1].starts_with("> "), "the composer does not follow: {rows:?}");
    assert_eq!(rows[rule].chars().count(), 80);
    let first = rows.iter().position(|r| *r == "> first").unwrap();
    let second = rows.iter().position(|r| *r == "> second").unwrap();
    assert_eq!(rows[second - 1], "", "no gap before the second turn: {rows:?}");
    assert_ne!(rows[first - 1], "> first");
    assert!(rows[0].starts_with(" first") && rows[0].trim_end().len() > 70, "no title bar: {rows:?}");
    assert!(rows[1..first].iter().all(|r| r.is_empty()), "{rows:?}");

    let short = strip(&a.frame(80, 7));
    assert!(!short.contains("────"), "a rule on a 7-row screen: {short:?}");
}

#[test]
fn an_empty_transcript_shows_a_welcome_row() {
    let mut a = app();
    let frame = strip(&a.frame(120, 24));
    assert!(frame.contains("imp · / for commands"), "{frame:?}");
    assert_eq!(frame.matches(tilde(&a.root).as_str()).count(), 1, "{frame:?}");
    a.push(Role::User, "hello");
    assert!(!strip(&a.frame(80, 24)).contains("/ for commands"));
    let narrow = strip(&app().frame(30, 24));
    assert!(narrow.contains("/ for commands"), "{narrow:?}");
}

#[test]
fn the_status_line_ends_with_the_directory_when_it_fits() {
    let a = app();
    let root = tilde(&a.root);
    let wide = a.status_line(100);
    assert!(wide.ends_with(&root), "{wide}");
    assert_eq!(wide.chars().count(), 100, "{wide}");
    assert!(wide.starts_with("deepseek-flash · "), "{wide}");
    let narrow = a.status_line(30);
    assert!(narrow.starts_with("deepseek-flash"), "{narrow}");
}

#[test]
fn the_status_line_shortens_a_long_directory() {
    let mut a = app();
    a.agents.clear();
    let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
    a.root = home.join("Projects/imp/src/providers");
    let left = "deepseek-flash · 0 tok (0%) · $0.00";
    let at = |a: &App, cols: usize| {
        let line = a.status_line(cols);
        assert!(line.starts_with(left) && line.chars().count() <= cols, "{line}");
        line
    };
    assert!(at(&a, 100).ends_with("~/Projects/imp/src/providers"));
    // The percent adds four columns to the left part, so the directory gives way sooner.
    assert!(at(&a, 54).ends_with("  …/providers"), "{}", a.status_line(54));
    assert_eq!(at(&a, 40), left, "the directory is dropped before the left part");
    assert_eq!(at(&a, 38), left, "6 columns left: left out");
}

#[test]
fn a_waiting_prompt_has_an_accent_bar_and_rule() {
    let mut a = colour_app();
    let (tx, _rx) = channel();
    let idle = a.frame(80, 20);
    a.run_command("rename", &tx);
    let asking = a.frame(80, 20);
    let accent = a.palette.paint(Role::Info, "─");
    let accent = &accent[..accent.find('─').unwrap()];
    assert!(asking.contains(&format!("{accent}{}", "─".repeat(80))), "{asking:?}");
    assert!(!idle.contains(&format!("{accent}{}", "─".repeat(80))), "{idle:?}");
    assert!(strip(&asking).contains("▌ new title: "), "{asking:?}");

    let mut plain = app();
    plain.run_command("rename", &tx);
    let frame = plain.frame(80, 20);
    assert!(frame.contains("▌ new title: "), "{frame:?}");
    assert!(!frame.contains("\x1b[38"), "colour with colour off: {frame:?}");
}

#[test]
fn yolo_paints_the_prompt_and_rule_in_the_error_colour() {
    let mut a = colour_app();
    let (tx, _rx) = channel();
    let error = a.palette.paint(Role::Error, "> ");
    let rule = a.palette.paint(Role::Error, &"─".repeat(80));
    let normal = a.frame(80, 20);
    assert!(!normal.contains(&error) && !normal.contains(&rule), "{normal:?}");
    a.run_command("mode yolo", &tx);
    let yolo = a.frame(80, 20);
    assert!(yolo.contains(&error), "glyph: {yolo:?}");
    assert!(yolo.contains(&rule), "rule: {yolo:?}");
    a.run_command("mode normal", &tx);
    assert!(!a.frame(80, 20).contains(&error), "yolo colour stayed");

    let mut plain = app();
    plain.agents.clear();
    let before = plain.frame(80, 20);
    plain.cfg.set_permissions(crate::config::Permissions::Yolo);
    // Only the status line names it; no colour appears.
    let after = plain.frame(80, 20);
    assert!(!after.contains("38;"), "colour with colour off");
    let rows = |f: &str| f.split("\r\n").map(str::to_string).collect::<Vec<_>>();
    let (b, a2) = (rows(&before), rows(&after));
    assert_eq!(b[..b.len() - 1], a2[..a2.len() - 1], "more than the status line changed");
    assert!(a2.last().unwrap().contains("deepseek-flash · yolo · "), "{after:?}");
}

#[test]
fn escapes_in_replies_and_tools_never_reach_the_terminal() {
    let mut a = app();
    a.push(Role::Assistant, "copy \u{1b}]52;c;aGk=\u{7}done");
    a.push(Role::Tool, "✓ run `ls`\n\u{1b}[31mred\u{1b}[0m");
    let frame = a.frame(80, 12);
    assert!(!frame.contains("]52;"), "{frame:?}");
    assert!(!frame.contains("[31m"), "{frame:?}");
    assert!(strip(&frame).contains("copy done"), "{frame:?}");
}

#[test]
fn frame_fits_the_terminal() {
    let mut a = app();
    a.push(Role::User, "a".repeat(200));
    a.push(Role::Assistant, "word ".repeat(100));
    let out = a.frame(40, 12);
    for line in out.lines() {
        let visible = strip(line);
        assert!(visible.chars().count() <= 40, "{visible:?}");
    }
}

#[test]
fn typed_output_keeps_its_colour() {
    let mut a = colour_app();
    a.push(Role::Tool, format!("{RAW}\u{1b}[31mred\u{1b}[0m \u{1b}[2Jplain"));
    let screen = a.frame(40, 10);
    assert!(screen.contains("\u{1b}[31mred\u{1b}[0m plain"), "{screen:?}");
    assert!(!screen.contains("\u{1b}[2J"), "a clear reached the screen");
    let mut plain = app();
    plain.push(Role::Tool, format!("{RAW}\u{1b}[31mred\u{1b}[0m"));
    let screen = plain.frame(40, 10);
    assert!(screen.contains("  red") && !screen.contains("\u{1b}[31m"), "{screen:?}");
}

#[test]
fn frames_reuse_rows_until_an_entry_changes() {
    let mut a = app();
    for i in 0..50 {
        a.push(Role::User, format!("question {i}"));
        a.push(Role::Assistant, format!("**answer** {i}\n\n- one\n- two"));
    }
    let full = a.frame(60, 20);
    assert_eq!(a.frame(60, 20), full, "a cached frame differs");
    a.drawn.clear();
    assert_eq!(a.frame(60, 20), full, "a fresh frame differs");

    // A marked slot survives while its entry is unchanged.
    a.drawn[0].1 = vec![(Role::Info, Line::Text("cached".into()))];
    a.frame(60, 20);
    assert!(matches!(&a.drawn[0].1[0].1, Line::Text(t) if t == "cached"), "re-rendered");
    a.entries[0].text.push('!');
    a.frame(60, 20);
    assert!(matches!(&a.drawn[0].1[0].1, Line::Text(t) if t == "> question 0!"), "stale row kept");
    // A new width renders everything again.
    a.drawn[2].1.clear();
    a.frame(50, 20);
    assert!(!a.drawn[2].1.is_empty());
}

#[test]
#[ignore]
fn time_a_long_transcript() {
    let mut a = app();
    for i in 0..400 {
        a.push(Role::User, format!("question {i} about the parser"));
        a.push(Role::Assistant, "**answer** with `code` and a list\n\n- one\n- two\n".repeat(3));
    }
    a.frame(80, 24);
    let t = Instant::now();
    for _ in 0..50 {
        a.entries.last_mut().unwrap().text.push_str(" more");
        a.frame(80, 24);
    }
    eprintln!("{:?} per streamed frame", t.elapsed() / 50);
}

#[test]
fn frame_shows_status_and_prompt() {
    let mut a = app();
    let out = a.frame(100, 20);
    assert!(out.contains("deepseek-flash"));
    assert!(out.contains("> "));
}

#[test]
fn frame_uses_crlf_for_every_row_boundary() {
    let mut a = app();
    a.push(Role::User, "hello");
    a.push(Role::Assistant, "hi");
    let out = a.frame(80, 10);
    assert!(
        !out.replace("\r\n", "").contains('\n'),
        "bare line feed in frame"
    );
}

#[test]
fn ctrl_e_expands_the_last_tool_call() {
    let mut a = app();
    a.apply(Ev::ToolStart {
        name: "bash".into(),
        summary: "run `ls -la`".into(),
        args: "{\"command\":\"ls -la\"}".into(),
    });
    a.apply(Ev::ToolEnd {
        ok: true,
        preview: "exit 0".into(),
        output: "total 8\nfoo".into(),
        detail: String::new(),
        ms: 4,
    });
    let (tx, _cmds) = channel();
    a.handle_key(Key::Ctrl('o'), &tx);
    assert!(!a.entries[0].text.contains("$ ls -la"), "Ctrl+O still expands");
    assert_eq!(a.entries.last().unwrap().text, "nothing waiting");
    a.handle_key(Key::Ctrl('e'), &tx);
    let text = a.entries[0].text.clone();
    assert!(text.contains("\n  $ ls -la"), "{text}");
    assert!(text.contains("total 8"), "{text}");
    a.handle_key(Key::Ctrl('e'), &tx);
    assert!(!a.entries[0].text.contains("$ ls -la"), "it did not collapse");
}

#[test]
fn inline_code_sits_on_its_tint_and_is_not_bold() {
    let mut a = colour_app();
    a.push(Role::Assistant, "could not `bash` it");
    let out = a.frame(60, 12);
    let [r, g, b] = crate::theme::tint(
        a.palette.theme.rgb(crate::theme::Slot::Code),
        a.palette.theme.rgb(crate::theme::Slot::Text),
    );
    let [x, y, z] = a.palette.theme.rgb(crate::theme::Slot::Text);
    let code = format!("\x1b[48;2;{r};{g};{b}m\x1b[38;2;{x};{y};{z}mbash\x1b[0m");
    assert!(out.contains(&code), "no tint: {out:?}");
    assert!(!out.contains("\x1b[1m"), "code is bold: {out:?}");
    // Mono has no tint, so code is reversed.
    a.palette = a.palette.with_theme(crate::theme::by_name("mono").unwrap());
    a.push(Role::Assistant, "and `sh`");
    assert!(a.frame(60, 12).contains("\x1b[7msh\x1b[0m"));
}

#[test]
fn markdown_structure_is_styled() {
    let mut a = colour_app();
    a.cfg.links = crate::config::Links::Always;
    a.push(Role::Assistant, "# Big one\n## Mid\n### Low\n- item\n> said\n[docs](https://example.com)");
    let out = a.frame(60, 20);
    let accent = |t: &str| a.palette.paint(Role::Info, t);
    assert!(out.contains(&format!("\x1b[1;4m{}", accent("Big"))), "h1: {out:?}");
    // The space inside the heading keeps its underline.
    assert!(out.contains(&format!("\x1b[1;4m{}", accent(" "))), "gap: {out:?}");
    assert!(out.contains(&format!("\x1b[1m{}", accent("Mid"))), "h2: {out:?}");
    assert!(out.contains(&format!("\x1b[1m{}", a.palette.paint(Role::Assistant, "Low"))), "h3: {out:?}");
    assert!(out.contains(&accent("• ")), "marker: {out:?}");
    assert!(out.contains(&format!("\x1b[3m{}", a.palette.paint(Role::Thinking, "said"))), "quote: {out:?}");
    assert!(out.contains(&format!("\x1b[4m{}", a.palette.paint(Role::Link, "docs"))), "link: {out:?}");
}

#[test]
fn interface_rows_carry_emphasis() {
    let mut a = colour_app();
    let (tx, _cmds) = channel();
    a.push(Role::Thinking, "pondering");
    let out = a.frame(60, 12);
    let thinking = a.palette.paint(Role::Thinking, "~ pondering");
    assert!(out.contains(&format!("\x1b[3m{thinking}")), "thinking: {out:?}");
    let colour = a.palette.paint(Role::Thinking, "").replace("\x1b[0m", "");
    // Command picker: title bold, other rows' descriptions muted.
    a.input = "/".into();
    a.cursor = 1;
    a.sync_menu();
    let out = a.frame(80, 30);
    assert!(out.contains("\x1b[1m"), "no bold title: {out:?}");
    let Some(Pending::Pick(p)) = &a.pending else { panic!("no picker") };
    let second = &p.items[p.matches()[1]].desc;
    let head: String = second.chars().take(10).collect();
    let at = out.find(&head).expect("description not shown");
    let before = &out[..at];
    assert!(before.rfind(&colour) > before.rfind("\x1b[0m"), "description not muted: {out:?}");
    // The unseen count is accent, the rest of the status line muted.
    a.pending = None;
    a.input.clear();
    for i in 0..20 {
        a.push(Role::User, format!("line {i}"));
    }
    a.frame(60, 8);
    a.handle_key(Key::PageUp, &tx);
    a.push(Role::User, "more");
    let out = a.frame(60, 8);
    let count = format!("↑ {}", a.unseen);
    assert!(out.contains(&a.palette.paint(Role::Info, &count)), "count: {out:?}");
}

#[test]
fn help_titles_are_bold_and_commands_accent() {
    let mut a = colour_app();
    let (tx, _cmds) = channel();
    a.run_command("help", &tx);
    let out = a.frame(80, 60);
    assert!(out.contains("\x1b[1mConversation\x1b[0m"), "{out:?}");
    assert!(out.contains(&a.palette.paint(Role::Info, "/compact")), "{out:?}");
    assert!(!out.contains("\x1b[36m"), "terminal cyan left in: {out:?}");
    // Without colour, /help has no escapes.
    let mut b = app();
    b.run_command("help", &tx);
    assert!(!b.frame(80, 60).contains("\x1b[1m"));
    // At Hackberry widths every help row arrives whole and flush left.
    for cols in [40, 54] {
        let mut c = colour_app();
        c.cols.store(cols as u16, Ordering::Relaxed);
        c.run_command("help", &tx);
        let want = crate::help::commands((cols - 2).max(40), true, false);
        let rows: Vec<String> = entry_lines(&c.entries[0], cols, false, false)
            .into_iter()
            .map(|(_, l)| match l {
                Line::Spans(s) => unpaint(&s.iter().map(|s| s.text.as_str()).collect::<String>()),
                _ => panic!("help is not raw"),
            })
            .collect();
        assert_eq!(rows.join("\n"), want.trim_end_matches('\n'), "at {cols}");
    }
}

#[test]
fn the_picker_highlight_has_a_background() {
    let mut a = colour_app();
    open_many(&mut a, 3);
    let out = a.frame(100, 30);
    let [r, g, b] = a.palette.theme.rgb(crate::theme::Slot::Select);
    let bg = format!("\x1b[48;2;{r};{g};{b}m");
    let at = out.find(&bg).expect("no select background");
    assert!(out[at..].contains("▸ profile 0"), "{out:?}");
    let border = a.palette.paint(Role::Border, "│");
    assert!(out.contains(&border), "the frame is not in the border colour");
}

#[test]
fn the_theme_picker_previews_and_esc_restores() {
    let mut a = colour_app();
    let (tx, _rx) = channel();
    let start = a.palette.theme_name();
    a.run_command("theme", &tx);
    a.sync_preview();
    a.handle_key(Key::Down, &tx);
    let shown = {
        let p = open_picker(&a);
        p.items[p.index].value.clone()
    };
    assert_ne!(shown, start);
    assert_eq!(a.palette.theme_name(), shown, "the highlight was not previewed");
    a.handle_key(Key::Esc, &tx);
    assert!(a.pending.is_none());
    assert_eq!(a.palette.theme_name(), start, "Esc did not restore");
}

#[test]
fn a_chosen_theme_stays_after_the_picker_closes() {
    // The Enter path saves to the state directory, so this drives the
    // same steps without touching it: choose, clear the start, close.
    let mut a = colour_app();
    let (tx, _rx) = channel();
    a.run_command("theme", &tx);
    a.handle_key(Key::Down, &tx);
    let chosen = a.palette.theme_name();
    a.preview_from = None;
    a.pending = None;
    a.sync_preview();
    assert_eq!(a.palette.theme_name(), chosen, "closing undid the choice");
}

#[test]
fn mono_previews_without_colour_and_back() {
    let mut a = colour_app();
    let mono = crate::theme::by_name("mono").unwrap();
    let off = a.palette.with_theme(mono);
    assert!(!off.on);
    assert!(off.with_theme(crate::theme::by_name("nord").unwrap()).on);
}

#[test]
fn mono_keeps_bold_and_reverse_but_no_colour() {
    let mut a = colour_app();
    a.palette = a.palette.with_theme(crate::theme::by_name("mono").unwrap());
    a.push(Role::Assistant, "a **loud** word");
    let out = a.frame(60, 12);
    assert!(out.contains("\x1b[1mloud\x1b[0m"), "{out:?}");
    assert!(!out.contains("38;") && !out.contains("48;"), "colour in mono: {out:?}");
    assert_eq!(a.palette.select("row"), "\x1b[7mrow\x1b[0m");
    // NO_COLOR and `never` still mean no escapes at all.
    let mut b = app();
    b.push(Role::Assistant, "a **loud** word");
    assert!(!b.frame(60, 12).contains("\x1b[1m"));
    assert_eq!(b.palette.select("row"), "row");
}

#[test]
fn a_fenced_block_stays_dim() {
    let mut a = colour_app();
    a.push(Role::Assistant, "```\nfn x() {}\n```");
    let out = a.frame(60, 12);
    assert!(
        out.contains("38;5;244") || out.contains("38;2;128;128;128"),
        "fence not dim: {out:?}"
    );
    assert!(!out.contains("\x1b[1mfn"), "fence got bold: {out:?}");
}

#[test]
fn assistant_rows_render_markdown() {
    let mut a = app();
    a.push(Role::Assistant, "**bold** and `code`\n\n- item one");
    let out = a.frame(60, 14);
    assert!(!out.contains("**"), "markers kept: {out:?}");
    assert!(!out.contains("`code`"), "backticks kept: {out:?}");
    assert!(out.contains("bold"), "{out:?}");
    assert!(out.contains("item one"), "{out:?}");
    assert!(out.contains('•'), "no bullet marker: {out:?}");
}

#[test]
fn frame_clears_below_the_status_row_before_drawing_it() {
    let mut a = app();
    let out = a.frame(40, 10);
    let row = &out[out.rfind("\r\n").expect("status row")..];
    let erase = row.find("\x1b[J").expect("nothing clears below the status");
    let text = row.find('·').expect("no status text");
    assert!(
        erase < text,
        "erase below runs after the status text: {row:?}"
    );
}
#[test]
fn rows_are_written_then_erased_never_blanked_first() {
    let mut a = app();
    a.push(Role::Info, "hi");
    let out = a.frame(40, 10);
    assert!(!out.contains("\x1b[2K"), "a row is blanked before its text: {out:?}");
    assert!(out.contains("hi\x1b[K") || out.contains("hi\x1b[0m\x1b[K"), "{out:?}");
    // A full-width row is not erased: the erase would take its last cell.
    assert!(!out.contains(&format!("{}\x1b[K", "─".repeat(40))), "{out:?}");
    assert!(out.starts_with("\x1b[?2026h") && out.ends_with("\x1b[?2026l"), "{out:?}");
}

#[test]
fn a_status_change_writes_one_row() {
    let mut a = app();
    a.push(Role::Assistant, "hello there");
    a.frame_changes(60, 12);
    a.status.tokens = 1234;
    let tick = a.frame_changes(60, 12);
    assert_eq!(tick.matches(";1H").count(), 1, "{tick:?}");
    assert!(!tick.contains("hello there"), "an unchanged row was written: {tick:?}");
    let same = a.frame_changes(60, 12);
    assert_eq!(same.matches(";1H").count(), 0, "{same:?}");
    let resized = a.frame_changes(61, 12);
    assert!(resized.contains("hello there") && resized.contains("\x1b[H"), "{resized:?}");
}

#[test]
fn composer_draws_one_row_per_line() {
    let mut a = app();
    a.input = "line one\nline two".into();
    a.cursor = a.input.len();
    let out = a.frame(60, 12).replace("\x1b[K", "");
    assert!(out.contains("> line one\r\n"), "{out:?}");
    assert!(out.contains("  line two"), "{out:?}");
    assert!(!out.contains('\u{0}'), "caret marker leaked into the frame");
}

#[test]
fn composer_keeps_the_caret_row_visible() {
    let mut a = app();
    a.input = (1..=12)
        .map(|i| format!("row {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    a.cursor = a.input.len();
    let out = a.frame(60, 12);
    assert!(out.contains("  row 12"), "last row missing: {out:?}");
    assert!(
        !out.contains("row 1\r\n"),
        "composer ignored its row budget"
    );
}

#[test]
fn composer_rows_fit_the_terminal() {
    let mut a = app();
    a.input = format!("{}\n{}", "x".repeat(90), "y".repeat(9));
    a.cursor = a.input.len();
    let out = a.frame(40, 10);
    for line in out.lines() {
        let visible = strip(line);
        assert!(visible.chars().count() <= 40, "{visible:?}");
    }
    assert!(
        !out.lines().filter(|l| l.contains('x') || l.contains('y')).any(|l| l.contains('…')),
        "a wrapped row was cut instead of wrapped"
    );
    assert_eq!(out.matches('x').count(), 90, "the draft lost characters");
}

#[test]
fn draft_rows_break_at_spaces_and_keep_every_byte() {
    let text = "the quick brown fox jumps over the lazy dog again and again and again";
    let rows = draft_rows(text, 20);
    assert!(rows.len() > 1);
    assert_eq!(rows[0].0, 0);
    assert_eq!(rows.last().unwrap().1, text.len());
    for pair in rows.windows(2) {
        assert_eq!(pair[0].1, pair[1].0, "rows leave a gap");
    }
    for &(s, e) in &rows[1..rows.len() - 1] {
        assert!(text[..e].ends_with(' '), "{:?} did not break at a space", &text[s..e]);
        assert!(text[s..e].chars().count() <= 20);
    }
}

#[test]
fn an_eighty_character_word_breaks_mid_word() {
    let text = "x".repeat(80);
    let rows = draft_rows(&text, 51);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|(s, e)| e - s <= 51));
    assert_eq!(rows[0].1, 51, "a word that cannot break was not cut at the edge");
}

#[test]
fn a_wide_draft_is_drawn_across_rows() {
    let mut a = app();
    a.input = "word ".repeat(24);
    a.cursor = a.input.len();
    let out = strip(&a.frame(54, 12));
    let lines: Vec<&str> = out.lines().map(|l| l.trim_end_matches('\r')).collect();
    assert!(
        lines.iter().all(|l| l.chars().count() <= 54),
        "a row is wider than the terminal: {out:?}"
    );
    assert!(out.contains("> word word"), "the first row lost its glyph: {out:?}");
    assert!(
        lines.iter().any(|l| l.starts_with("  word")),
        "no continuation row: {out:?}"
    );
    assert!(out.matches("word").count() >= 20, "the draft lost words: {out:?}");
}

#[test]
fn up_moves_inside_a_wrapped_line() {
    let mut a = app();
    let (tx, _cmds) = channel();
    a.input = "word ".repeat(30);
    a.cursor = a.input.len();
    a.cols.store(54, Ordering::Relaxed);
    let before = a.draft_rows_now();
    let (row, _) = a.drawn_caret(&before);
    assert_eq!(row, before.len() - 1, "the caret is not on the last row");
    a.handle_key(Key::Up, &tx);
    let (row, _) = a.drawn_caret(&before);
    assert_eq!(row, before.len() - 2, "Up did not move one drawn row");
    assert_eq!(a.input.len(), 150, "Up changed the draft");
}

#[test]
fn down_moves_inside_a_wrapped_line() {
    let mut a = app();
    let (tx, _cmds) = channel();
    a.input = "word ".repeat(30);
    a.cursor = 0;
    a.cols.store(54, Ordering::Relaxed);
    let rows = a.draft_rows_now();
    assert_eq!(a.drawn_caret(&rows).0, 0);
    a.handle_key(Key::Down, &tx);
    assert_eq!(a.drawn_caret(&rows).0, 1, "Down did not move one drawn row");
    assert_eq!(a.cursor, rows[1].0, "Down did not keep the column");
    assert_eq!(a.input.len(), 150, "Down changed the draft");
}

#[test]
fn up_on_the_top_row_recalls_history() {        let mut a = app();
    let (tx, _cmds) = channel();
    a.input = "x".repeat(120);
    a.cursor = 0;
    a.cols.store(54, Ordering::Relaxed);
    a.history.push("earlier".into());
    a.handle_key(Key::Up, &tx);
    assert_eq!(a.input, "earlier", "Up in the first row did not reach history");
}

#[test]
fn no_colour_emits_no_colour_escapes() {
    let mut a = app();
    a.push(Role::User, "hello");
    a.push(Role::Assistant, "hi");
    let out = a.frame(80, 10);
    assert!(!out.contains("38;"), "colour escaped in no-colour mode");
}

#[test]
fn colour_roles_use_256_and_truecolor() {
    let plain = Palette {
        on: true,
        allowed: true,
        truecolor: false,
        theme: &crate::theme::THEMES[0],
    };
    assert!(plain.paint(Role::Error, "x").contains("38;5;"));
    let rgb = Palette {
        on: true,
        allowed: true,
        truecolor: true,
        theme: &crate::theme::THEMES[0],
    };
    assert!(rgb.paint(Role::Error, "x").contains("38;2;"));
    let off = Palette {
        on: false,
        allowed: false,
        truecolor: true,
        theme: &crate::theme::THEMES[0],
    };
    assert_eq!(off.paint(Role::Error, "x"), "x");
}

#[test]
fn thinking_and_tool_lines_render() {
    let mut a = app();
    a.push(Role::Thinking, "considering options");
    a.push(Role::Tool, "bash  ls -la");
    let out = a.frame(80, 12);
    assert!(out.contains("~ considering"), "{out}");
    assert!(out.contains("bash  ls -la"), "{out}");
}

#[test]
fn the_spinner_glyph_stays_upright() {
    let mut a = colour_app();
    let (tx, _rx) = channel();
    a.input = "go".into();
    a.cursor = 2;
    a.handle_key(Key::Enter, &tx);
    let out = a.frame(80, 12);
    let glyph = spinner_frame(a.spinner).to_string();
    let line = out
        .lines()
        .find(|l| l.contains("thinking ·"))
        .expect("the thinking row");
    let at = line.find(&glyph).expect("the spinner");
    let italic = line.find("\x1b[3m").expect("no italic on the row");
    assert!(italic > at, "the spinner is italic: {line:?}");
}

#[test]
fn question_picker_renders_choices() {
    let mut a = app();
    let (tx, _rx) = channel();
    a.pending = Some(Pending::Question {
        q: "which database?".into(),
        opts: vec!["sqlite".into(), "postgres".into()],
        reply: tx,
        mode: AskMode::Choose,
        typed: String::new(),
    });
    let out = a.frame(100, 12).replace("\x1b[K", "");
    assert!(out.contains("which database?\r\n"), "{out}");
    assert!(out.contains("▌ [2] postgres"), "{out}");
    assert!(out.contains("[1] sqlite"), "{out}");
    assert!(out.contains("type to answer"), "{out}");
}

#[test]
fn the_active_step_is_always_visible() {
    let mut a = app();
    let (tx, _rx) = channel();
    a.input = "go".into();
    a.cursor = 2;
    a.handle_key(Key::Enter, &tx);
    assert!(strip(&a.frame(65, 12)).contains("thinking · 0s"), "no thinking row");

    a.apply(Ev::ToolStart { name: "bash".into(), summary: "Run test suite".into(), args: String::new() });
    assert!(!strip(&a.frame(65, 12)).contains("thinking"), "thinking beside a running tool");
    let (reply, _r) = channel();
    let command = format!("python3 -m unittest {} 2>&1 | tail -5", "tests/test_cart.py ".repeat(14));
    a.apply(Ev::Confirm { what: format!("run `{command}`"), reply, always: true });
    let out = strip(&a.frame(65, 20));
    assert!(out.contains("▌ Run test suite · waiting for you"), "{out}");
    assert!(out.contains("$ python3 -m unittest"), "{out}");
    assert!(out.contains("tests/test_cart.py tests/test_cart.py"), "{out}");
    for line in out.split("\r\n") {
        assert!(line.chars().filter(|c| *c != '\u{7}').count() <= 65, "{line:?}");
    }
}

#[test]
fn a_task_reports_its_meter_and_finish_line() {
    assert_eq!(
        task_line(Duration::from_secs(42), 5, 16_560, 1_440, 1_400, Some(0.004)),
        "42s · 5 steps · 18K in (92% cached) · 1.4K out · $0.004"
    );
    assert_eq!(task_line(Duration::from_secs(72), 1, 0, 0, 0, None), "1m12s · 1 step");

    let mut a = app();
    let (tx, _rx) = channel();
    a.input = "go".into();
    a.cursor = 2;
    a.handle_key(Key::Enter, &tx);
    a.apply(Ev::ToolStart { name: "bash".into(), summary: "ls".into(), args: String::new() });
    a.apply(Ev::Usage { tokens: None, hit: 900, miss: 100, output: 2100, model: None });
    let status = strip(&a.status_line(80));
    assert!(status.ends_with("0s · 1 step · ↓2.1K"), "{status}");
    a.apply(Ev::Done(None));
    let last = a.entries.last().unwrap();
    assert!(last.text.starts_with("0s · 1 step · 1K in (90% cached) · 2.1K out"), "{}", last.text);
    assert!(!strip(&a.status_line(80)).contains("step"), "the meter stayed");

    a.verbosity = Verbosity::Low;
    a.begin_task();
    a.status.streaming = true;
    let n = a.entries.len();
    a.apply(Ev::Done(None));
    assert_eq!(a.entries.len(), n, "a finish line at verbosity low");
}

#[test]
fn a_usage_event_moves_the_context_mid_turn() {
    let mut a = app();
    let (tx, _rx) = channel();
    a.input = "go".into();
    a.cursor = 2;
    a.handle_key(Key::Enter, &tx);
    a.apply(Ev::Usage { tokens: Some(7700), hit: 10, miss: 90, output: 20, model: None });
    let status = strip(&a.status_line(80));
    assert!(status.contains("7.7K tok"), "{status}");
    assert!(a.status.streaming, "the event ended the turn");
}

#[test]
fn usage_without_tokens_keeps_the_context() {
    let mut a = app();
    a.apply(Ev::Usage { tokens: Some(5000), hit: 1, miss: 2, output: 3, model: None });
    a.apply(Ev::Usage { tokens: None, hit: 10, miss: 20, output: 30, model: None });
    assert_eq!((a.status.tokens, a.status.hit, a.status.miss, a.status.output), (5000, 11, 22, 33));
}

#[test]
fn a_suggestion_is_taken_with_right_and_dropped_by_typing() {
    let mut a = app();
    let (tx, _rx) = channel();
    a.apply(Ev::Suggest("run the tests".into()));
    let shown = strip(&a.frame(60, 10));
    assert!(shown.contains(">  run the tests"), "not drawn");
    assert!(!shown.contains("run the tests  Tab"), "the key hint is back: {shown:?}");
    a.handle_key(Key::Right, &tx);
    assert_eq!(a.input, "run the tests");
    assert_eq!(a.cursor, a.input.len());

    let mut a = app();
    a.apply(Ev::Suggest("run the tests".into()));
    a.handle_key(Key::Char('n'), &tx);
    assert_eq!(a.input, "n");
    assert!(!strip(&a.frame(60, 10)).contains("run the tests"), "still drawn");

    let mut a = app();
    a.apply(Ev::Suggest("x".into()));
    let out = strip(&a.frame(60, 10));
    assert!(out.contains("→ accept"), "no rule hint: {out:?}");
    a.handle_key(Key::Enter, &tx);
    assert_eq!(a.input, "x", "Enter did not fill the draft");
    assert!(_rx.try_recv().is_err(), "Enter sent the suggestion");

    let mut a = app();
    a.input = "draft".into();
    a.apply(Ev::Suggest("late".into()));
    assert!(a.status.suggestion.is_none(), "a late suggestion was kept");
}

#[test]
fn long_choices_wrap_instead_of_leaving_the_screen() {
    let mut a = app();
    let (tx, _rx) = channel();
    let long = "imp's agent loop (idle → streaming → tool call → tool end → streaming again)";
    a.pending = Some(Pending::Question {
        q: "Which state machine should I chart?".into(),
        opts: vec![long.into(), "A job's lifecycle".into(), "Something else".into()],
        reply: tx,
        mode: AskMode::Choose,
        typed: String::new(),
    });
    let out = a.frame(40, 20);
    for line in out.split("\r\n") {
        assert!(strip(line).chars().count() <= 40, "{line:?}");
    }
    let text = strip(&out);
    assert!(text.contains("again)"), "the end of choice 1 was cut: {text}");
    assert!(text.contains("[3] Something else"), "{text}");
    assert!(text.contains("type to answer"), "{text}");
}

#[test]
fn yes_no_shortcuts_answer() {
    let mut a = app();
    let (tx, rx) = channel();
    a.pending = Some(Pending::Question {
        q: "delete?".into(),
        opts: vec!["Yes".into(), "No".into()],
        reply: tx,
        mode: AskMode::Choose,
        typed: String::new(),
    });
    let (cmd_tx, _cmd_rx) = channel();
    a.handle_key(Key::Char('y'), &cmd_tx);
    assert_eq!(rx.recv().unwrap().as_deref(), Some("Yes"));
}

#[test]
fn typing_starts_a_custom_answer() {
    let mut a = app();
    let (tx, rx) = channel();
    a.pending = Some(Pending::Question {
        q: "which?".into(),
        opts: vec!["Python".into(), "Rust".into()],
        reply: tx,
        mode: AskMode::Choose,
        typed: String::new(),
    });
    assert!(strip(&a.frame(60, 20)).contains("type to answer   Esc stop"));
    let (cmd_tx, _cmd_rx) = channel();
    a.handle_key(Key::Enter, &cmd_tx);
    for c in "yes, go".chars() {
        a.handle_key(Key::Char(c), &cmd_tx);
    }
    assert!(rx.try_recv().is_err(), "y picked a choice that is not Yes");
    assert!(strip(&a.frame(60, 20)).contains("> yes, go"), "the first letter was lost");
    a.handle_key(Key::Enter, &cmd_tx);
    assert_eq!(rx.recv().unwrap().as_deref(), Some("yes, go"));
}

#[test]
fn an_empty_answer_is_not_sent() {
    let mut a = app();
    let (tx, rx) = channel();
    a.pending = Some(Pending::Question {
        q: "which?".into(),
        opts: vec!["A".into(), "B".into(), "C".into()],
        reply: tx,
        mode: AskMode::Custom,
        typed: String::new(),
    });
    let (cmd_tx, _cmd_rx) = channel();
    a.handle_key(Key::Enter, &cmd_tx);
    assert!(rx.try_recv().is_err(), "an empty answer was sent");
    assert!(matches!(a.pending, Some(Pending::Question { mode: AskMode::Custom, .. })));
}

/// An open three-choice question and the channel its answer arrives on.
fn asked(a: &mut App) -> Receiver<Option<String>> {
    let (tx, rx) = channel();
    a.pending = Some(Pending::Question {
        q: "which?".into(),
        opts: vec!["A".into(), "B".into(), "C".into()],
        reply: tx,
        mode: AskMode::Choose,
        typed: String::new(),
    });
    rx
}

#[test]
fn the_answer_row_edits_like_the_composer() {
    let mut a = app();
    let rx = asked(&mut a);
    let (tx, _cmds) = channel();
    for c in "use the old parser".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    a.handle_key(Key::Ctrl('w'), &tx);
    a.handle_key(Key::Word(Move::Left), &tx);
    a.handle_key(Key::Char('x'), &tx);
    a.clip = "new ".into();
    a.handle_key(Key::Home, &tx);
    a.handle_key(Key::Ctrl('v'), &tx);
    let shown = strip(&a.frame(60, 20));
    assert!(shown.contains("> new  use the xold"), "the caret is not after the paste: {shown}");
    a.handle_key(Key::Enter, &tx);
    assert_eq!(rx.recv().unwrap().as_deref(), Some("new use the xold "));
}

#[test]
fn space_starts_an_answer_with_a_digit() {
    let mut a = app();
    let rx = asked(&mut a);
    let (tx, _cmds) = channel();
    a.handle_key(Key::Char(' '), &tx);
    for c in "3 files".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    assert!(rx.try_recv().is_err(), "a digit picked a choice");
    a.handle_key(Key::Enter, &tx);
    assert_eq!(rx.recv().unwrap().as_deref(), Some("3 files"));
}

#[test]
fn status_reports_tokens_and_cost() {
    let mut a = app();
    a.agents.clear();
    a.status.tokens = 3400;
    a.status.hit = 900;
    a.status.miss = 100;
    a.status.output = 50;
    let line = a.status_line(120);
    assert!(line.contains("3.4K tok"), "{line}");
    let cost =
        crate::pricing::cost("deepseek-flash", crate::pricing::now(), 900, 100, 50).unwrap();
    let shown = if cost < 0.1 { format!("${cost:.3}") } else { format!("${cost:.2}") };
    assert!(line.contains(&shown), "{line}");
    assert!(!line.contains("cache"), "{line}");
    assert!(!line.contains("normal"), "{line}");
}

#[test]
fn status_abbreviates_large_token_counts() {
    let mut a = app();
    a.status.tokens = 940;
    assert!(a.status_line(120).contains("940 tok"));
    a.status.tokens = 1_200_000;
    assert!(a.status_line(120).contains("1.2M tok"));
    assert_eq!(compact_tokens(12_345), "12.3K");
    assert_eq!(compact_tokens(1_000), "1K");
}

#[test]
fn status_sums_cost_across_turns() {
    let mut a = app();
    a.apply(Ev::Usage {
        tokens: Some(100),
        hit: 900,
        miss: 100,
        output: 50,
        model: None,
    });
    a.apply(Ev::Usage {
        tokens: Some(200),
        hit: 400,
        miss: 200,
        output: 25,
        model: None,
    });
    assert_eq!(a.status.hit, 1300);
    assert_eq!(a.status.miss, 300);
    assert_eq!(a.status.output, 75);
    let total =
        crate::pricing::cost("deepseek-flash", crate::pricing::now(), 1300, 300, 75).unwrap();
    let line = a.status_line(120);
    assert!(line.contains(&format!("${total:.2}")), "{line}");
}

#[test]
fn an_unknown_model_shows_tokens_but_no_cost() {
    let mut a = app();
    a.cfg.set_provider(crate::providers::Provider::OpenAi);
    a.status.tokens = 100;
    a.status.hit = 90;
    a.status.miss = 10;
    a.status.output = 5;
    let line = a.status_line(120);
    assert!(line.contains("100 tok"), "{line}");
    assert!(!line.contains('$'), "{line}");
}

#[test]
fn status_shows_the_served_model_when_it_differs() {
    let mut a = app();
    a.status.served = "deepseek-flash-2026-11".into();
    assert!(
        a.status_line(120)
            .contains("deepseek-flash (deepseek-flash-2026-11)"),
        "{}",
        a.status_line(120)
    );
    a.status.served = "deepseek-flash".into();
    assert!(!a.status_line(120).contains("deepseek-flash ("), "{}", a.status_line(120));
}

#[test]
fn an_idle_app_does_not_render() {
    let mut a = app();
    a.dirty = false;
    assert!(!a.wants_render(false), "an idle screen queued a frame");
    assert!(a.wants_render(true), "a key did not queue a frame");
    a.dirty = true;
    assert!(
        a.wants_render(false),
        "a dirty screen did not queue a frame"
    );
}

#[test]
fn the_spinner_runs_at_ten_frames_a_second() {
    let mut a = app();
    a.status.streaming = true;
    a.last_redraw = Instant::now() - Duration::from_millis(101);
    assert!(!a.spinner_due(), "a turn without a tool ticked the spinner");

    a.apply(Ev::ToolStart {
        name: "bash".into(),
        summary: "run `sleep 1`".into(),
        args: "{}".into(),
    });
    a.last_redraw = Instant::now();
    assert!(!a.spinner_due(), "spinner ran faster than 100 ms");
    a.last_redraw = Instant::now() - Duration::from_millis(101);
    assert!(a.spinner_due(), "spinner stalled after 100 ms");

    let before = a.entries[0].text.clone();
    a.tick_spinner();
    assert_ne!(a.entries[0].text, before, "the row did not advance");
    assert!(a.entries[0].text.starts_with(spinner_frame(a.spinner)));

    a.apply(Ev::ToolEnd {
        ok: true,
        preview: "done".into(),
        output: "done".into(),
        detail: String::new(),
        ms: 7,
    });
    // The model is thinking about the next step, so the thinking row spins.
    let done = a.entries[0].text.clone();
    assert!(a.spinner_due(), "the thinking row did not spin");
    a.tick_spinner();
    assert_eq!(a.entries[0].text, done, "a finished tool kept ticking");
    a.apply(Ev::Done(None));
    assert!(!a.spinner_due(), "an idle session ticked");
}

#[test]
fn the_status_line_has_no_spinner() {
    let mut a = app();
    a.status.streaming = true;
    a.apply(Ev::ToolStart {
        name: "bash".into(),
        summary: "run `sleep 1`".into(),
        args: "{}".into(),
    });
    let line = a.status_line(80);
    for frame in spinner_frames().chars() {
        assert!(
            !line.contains(frame),
            "status line carried {frame:?}: {line:?}"
        );
    }
    assert!(line.starts_with("deepseek-flash"), "{line:?}");
}

#[test]
fn a_resize_is_noticed_without_a_timer() {
    let mut a = app();
    a.last_size = (80, 24);
    assert!(!a.resized(80, 24));
    assert!(a.resized(81, 24));
    assert!(a.resized(80, 25));
}

#[test]
fn mode_changes_the_shared_mode_immediately() {
    let mut a = app();
    a.status.streaming = true;
    let (tx, rx) = channel();
    assert_eq!(a.cfg.live_permissions(), crate::config::Permissions::Normal);
    a.run_command("mode yolo", &tx);
    assert_eq!(a.cfg.live_permissions(), crate::config::Permissions::Yolo);
    assert!(matches!(rx.try_recv(), Ok(Cmd::SetPermissions(crate::config::Permissions::Yolo))));
    a.run_command("mode strict", &tx);
    assert_eq!(a.cfg.live_permissions(), crate::config::Permissions::Strict);
    assert!(a.entries.last().unwrap().text.starts_with("mode: strict"), "no mode line");
    // Queued commands each carry their target; the last wins.
    a.run_command("mode yolo", &tx);
    a.run_command("mode normal", &tx);
    let sent: Vec<_> = rx.try_iter().filter_map(|c| match c {
        Cmd::SetPermissions(m) => Some(m),
        _ => None,
    }).collect();
    assert_eq!(sent.last(), Some(&crate::config::Permissions::Normal));
}

#[test]
fn mode_rejects_an_unknown_name() {
    let mut a = app();
    let (tx, rx) = channel();
    a.run_command("mode wild", &tx);
    assert_eq!(a.cfg.live_permissions(), crate::config::Permissions::Normal);
    assert!(rx.try_recv().is_err(), "an unknown mode reached the worker");
    let last = a.entries.last().unwrap();
    assert_eq!(last.role, Role::Error);
    assert_eq!(last.text, "unknown mode wild; use normal, strict, or yolo");
}

#[test]
fn mode_picker_opens_on_the_current_mode() {
    let mut a = app();
    let (tx, _rx) = channel();
    a.cfg.set_permissions(crate::config::Permissions::Strict);
    a.run_command("mode", &tx);
    let Some(Pending::Pick(p)) = &a.pending else { panic!("no picker") };
    assert_eq!(
        p.items.iter().map(|i| i.value.as_str()).collect::<Vec<_>>(),
        ["normal", "auto", "strict", "yolo", "add-rule"]
    );
    assert_eq!(p.items[p.index].value, "strict");
    assert!(p.items[2].desc.starts_with("current"), "{}", p.items[2].desc);
    assert!(!p.items[0].desc.contains("current"));
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Enter, &tx);
    assert_eq!(a.cfg.live_permissions(), crate::config::Permissions::Yolo);
    // Tab on a partial command opens the values, like other value commands.
    for c in "/mo".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    a.handle_key(Key::Tab, &tx);
    let Some(Pending::Pick(p)) = &a.pending else { panic!("Tab did not open the mode values") };
    assert_eq!(p.title, "mode");
}

#[test]
fn a_result_that_restates_the_step_is_dropped() {
    assert!(echoes("edit x.txt (1 change)", "edited x.txt (1 change)"));
    assert!(!echoes("run `ls`", "exit 0"));
}

#[test]
fn shell_command_sends_the_attached_runner() {
    let mut a = app();
    let (tx, rx) = channel();
    a.run_command("shell", &tx);
    assert!(matches!(rx.try_recv(), Ok(Cmd::Shell)));
}

#[test]
fn model_command_sets_the_session_and_records_recents() {
    let dir = scratch("models");
    let config = dir.join("config");
    let mut a = app();
    a.config_path = config.clone();
    a.models_path = config.with_file_name("models");
    let (tx, rx) = channel();
    a.run_command("model deepseek-v4-pro", &tx);
    assert!(matches!(rx.try_recv(), Ok(Cmd::SetModel(name)) if name == "deepseek-v4-pro"));
    assert_eq!(a.cfg.model, "deepseek-v4-pro");

    a.run_command("model other", &tx);
    assert_eq!(
        load_models(&config.with_file_name("models")),
        vec!["other", "deepseek-v4-pro"]
    );

    a.run_command("model", &tx);
    let picker = open_picker(&a);
    assert_eq!(picker.kind, PickKind::Model);
    let labels: Vec<&str> = picker.items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels[0], "other");
    assert!(picker.items[0].value.ends_with("::other"), "{:?}", picker.items[0].value);
    assert!(picker.items[0].desc.starts_with("current"), "{:?}", picker.items[0].desc);
    assert_eq!(labels[1], "deepseek-flash");
    assert!(picker.items[1].desc.ends_with("default"), "{:?}", picker.items[1].desc);
    assert!(picker.items.iter().all(|i| i.value.contains("::")), "a row with no provider: {labels:?}");
    assert!(labels.iter().all(|l| !l.contains("(current)")), "{labels:?}");
}

#[test]
fn pickers_mark_the_current_entry_one_way() {
    let mut a = app();
    open_agent_picker(&mut a);
    let picker = open_picker(&a);
    let current = &picker.items[picker.index];
    assert_eq!(current.desc, "current · default profile");
    assert_eq!(picker.items[1].desc, "reviewer profile");
    a.pending = None;
    let (tx, _rx) = channel();
    a.run_command("theme", &tx);
    let picker = open_picker(&a);
    let marked: Vec<&str> = picker
        .items
        .iter()
        .filter(|i| i.desc.starts_with("current"))
        .map(|i| i.label.as_str())
        .collect();
    assert_eq!(marked, vec![a.palette.theme_name()]);
}

#[test]
fn the_model_picker_lists_the_provider_catalog() {
    let mut a = app();
    let (tx, rx) = channel();
    a.run_command("model", &tx);
    assert!(matches!(rx.try_recv(), Ok(Cmd::ListModels)));
    let picker = open_picker(&a);
    let labels: Vec<&str> = picker.items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.len() >= 3, "only {} model rows", labels.len());
    assert!(labels.contains(&"deepseek-chat"), "{labels:?}");
    assert!(labels.contains(&"deepseek-reasoner"), "{labels:?}");
}

#[test]
fn a_live_model_list_replaces_the_catalog_rows() {
    let mut a = app();
    let (tx, _rx) = channel();
    a.run_command("model", &tx);
    if let Some(Pending::Pick(picker)) = &mut a.pending {
        picker.filter = "be".into();
    }
    a.apply(Ev::Models(vec!["alpha".into(), "beta".into()]));
    let picker = open_picker(&a);
    let labels: Vec<&str> = picker.items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"alpha"), "{labels:?}");
    assert!(labels.contains(&"beta"), "{labels:?}");
    assert!(!labels.contains(&"deepseek-chat"), "catalog row stayed: {labels:?}");
    assert_eq!(picker.filter, "be", "the filter was lost");
}

#[test]
fn switching_provider_swaps_the_model_catalog() {
    let mut a = app();
    let (tx, _rx) = channel();
    a.run_command("provider use openai", &tx);
    assert!(a.available_models.iter().any(|m| m == "gpt-4.1"));
    a.run_command("model", &tx);
    let labels: Vec<&str> = open_picker(&a)
        .items
        .iter()
        .map(|i| i.label.as_str())
        .collect();
    assert!(labels.iter().any(|l| l.starts_with("gpt-4.1")), "{labels:?}");
}

#[test]
fn queue_adds_lists_pops_deletes_clears_and_pops_all() {
    let mut a = app();
    let (tx, _rx) = channel();
    a.run_command("queue check the tests", &tx);
    a.run_command("queue write the docs", &tx);
    a.run_command("queue", &tx);
    let listed = &a.entries.last().unwrap().text;
    assert!(listed.contains("1. check the tests"), "{listed}");
    assert!(listed.contains("2. write the docs"), "{listed}");

    a.run_command("queue pop 1", &tx);
    assert_eq!(a.input, "check the tests");
    assert_eq!(a.queued.len(), 1);

    a.run_command("queue delete 1", &tx);
    assert!(a.queued.is_empty());
    assert_eq!(a.input, "check the tests", "delete touched the composer");

    a.run_command("queue first", &tx);
    a.run_command("queue second", &tx);
    a.run_command("queue pop all", &tx);
    assert_eq!(a.input, "check the tests\n\nfirst\n\nsecond");
    assert!(a.queued.is_empty());

    a.run_command("queue pop 9", &tx);
    assert_eq!(a.entries.last().unwrap().role, Role::Error);
    a.run_command("queue one", &tx);
    a.run_command("queue clear", &tx);
    assert!(a.queued.is_empty());
}

#[test]
fn rename_command_prompts_or_sends_the_title() {
    let mut a = app();
    let (tx, rx) = channel();
    a.run_command("rename fix the parser", &tx);
    assert!(
        matches!(rx.try_recv(), Ok(Cmd::RenameCurrent(title)) if title == "fix the parser")
    );

    a.run_command("rename", &tx);
    assert!(matches!(a.pending, Some(Pending::RenameCurrent { .. })));
    for c in "new title".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    a.handle_key(Key::Enter, &tx);
    assert!(matches!(rx.try_recv(), Ok(Cmd::RenameCurrent(title)) if title == "new title"));
}

fn provider_app() -> (App, std::path::PathBuf) {
    let dir = scratch("providers");
    let config = dir.join("config");
    std::fs::write(&config, "# keep me\nmodel = old\n").unwrap();
    let mut a = app();
    a.config_path = config.clone();
    (a, config)
}

#[test]
fn provider_add_edits_the_managed_block() {
    let (mut a, config) = provider_app();
    let (tx, _rx) = channel();
    a.run_command("provider add work", &tx);
    assert!(matches!(a.pending, Some(Pending::ProviderForm { .. })));
    for key in ["http://box:8080/v1", "local-coder"] {
        for c in key.chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        a.handle_key(Key::Enter, &tx);
    }
    // The dialect steps from the list: deepseek, then openai.
    a.handle_key(Key::Right, &tx);
    a.handle_key(Key::Right, &tx);
    a.handle_key(Key::Enter, &tx);
    for c in "WORK_KEY".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    // key_env, then models, then the key row, which saves when empty.
    a.handle_key(Key::Enter, &tx);
    a.handle_key(Key::Enter, &tx);
    a.handle_key(Key::Enter, &tx);
    assert!(a.pending.is_none(), "the form stayed open");
    let text = std::fs::read_to_string(&config).unwrap();
    assert!(text.starts_with("# keep me\nmodel = old\n"), "{text}");
    assert!(text.contains("provider.work.base = http://box:8080/v1"), "{text}");
    assert!(text.contains("provider.work.dialect = openai"), "{text}");
    assert_eq!(a.cfg.named.len(), 1);

    a.run_command("provider edit work", &tx);
    for _ in 0..2 {
        a.handle_key(Key::Enter, &tx);
    }
    // openai steps on through anthropic and gemini to ollama.
    for _ in 0..3 {
        a.handle_key(Key::Right, &tx);
    }
    a.handle_key(Key::Enter, &tx);
    a.handle_key(Key::Enter, &tx);
    a.handle_key(Key::Enter, &tx);
    a.handle_key(Key::Enter, &tx);
    let text = std::fs::read_to_string(&config).unwrap();
    assert!(text.contains("provider.work.dialect = ollama"), "{text}");

    a.run_command("provider delete work", &tx);
    let text = std::fs::read_to_string(&config).unwrap();
    assert!(!text.contains("provider.work"), "{text}");
    assert!(text.contains("# keep me"), "{text}");
}

#[test]
fn provider_use_reaches_the_worker_and_delete_refuses_the_active_one() {
    let (mut a, _config) = provider_app();
    let (tx, rx) = channel();
    a.run_command("provider add work", &tx);
    for key in ["http://box/v1", "m"] {
        for c in key.chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        a.handle_key(Key::Enter, &tx);
    }
    a.handle_key(Key::Right, &tx);
    a.handle_key(Key::Right, &tx);
    a.handle_key(Key::Enter, &tx);
    for c in "K".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    a.handle_key(Key::Enter, &tx);
    // key_env, then models, then the key row, which saves when empty.
    a.handle_key(Key::Enter, &tx);
    a.handle_key(Key::Enter, &tx);
    a.handle_key(Key::Enter, &tx);

    a.run_command("provider use work", &tx);
    match rx.try_recv() {
        Ok(Cmd::UseProvider {
            name,
            dialect,
            base,
            model,
            key_env,
        }) => {
            assert_eq!(name, "work");
            assert_eq!(dialect, "openai");
            assert_eq!(base, "http://box/v1");
            assert_eq!(model, "m");
            assert_eq!(key_env, "K");
        }
        other => panic!("no use command: {}", other.is_ok()),
    }
    assert_eq!(a.cfg.active_name, "work");

    a.run_command("provider delete work", &tx);
    assert_eq!(a.entries.last().unwrap().role, Role::Error);
    assert_eq!(a.cfg.named.len(), 1, "the active profile was deleted");
}

#[test]
fn a_value_picker_opens_after_the_command_and_space() {
    let mut a = app();
    let (tx, _rx) = channel();
    for c in "/theme ".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    let picker = open_picker(&a);
    assert_eq!(picker.kind, PickKind::Theme);
    assert_eq!(picker.prefix.as_deref(), Some("/theme "));
    assert!(picker.filter.is_empty());

    for c in "gru".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    let picker = open_picker(&a);
    assert_eq!(picker.filter, "gru");
    assert_eq!(a.input, "/theme gru", "the composer did not mirror");
    let labels: Vec<&str> = picker
        .matches()
        .iter()
        .map(|&i| picker.items[i].label.as_str())
        .collect();
    assert_eq!(labels, vec!["gruvbox-dark", "gruvbox-light"], "{labels:?}");

    a.handle_key(Key::Esc, &tx);
    assert!(a.pending.is_none());
    assert_eq!(a.input, "/theme gru", "Esc discarded the typed text");
}

#[test]
fn bare_theme_and_provider_open_pickers_on_the_current_entry() {
    let mut a = app();
    let (tx, _rx) = channel();
    a.run_command("theme", &tx);
    let picker = open_picker(&a);
    assert_eq!(picker.kind, PickKind::Theme);
    let current = a.palette.theme_name();
    assert_eq!(picker.items[picker.index].value, current);
    a.pending = None;
    a.run_command("provider", &tx);
    let picker = open_picker(&a);
    assert_eq!(picker.kind, PickKind::Provider);
    assert_eq!(picker.items[picker.index].value, a.cfg.active_name);
}

#[test]
fn a_space_in_a_value_filter_hands_the_line_back() {
    let mut a = app();
    a.config_path = scratch("providers-space").join("config");
    let (tx, _rx) = channel();
    for c in "/provider add work".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    assert!(a.pending.is_none(), "a picker kept the free text");
    assert_eq!(a.input, "/provider add work");
    a.handle_key(Key::Enter, &tx);
    assert!(
        matches!(a.pending, Some(Pending::ProviderForm { .. })),
        "Enter did not start the provider form"
    );
}

#[test]
fn a_value_picker_applies_and_clears_the_line() {
    let mut a = app();
    let (tx, _rx) = channel();
    for c in "/verbosity ".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    a.handle_key(Key::Char('h'), &tx);
    a.handle_key(Key::Enter, &tx);
    assert_eq!(a.verbosity, Verbosity::High, "Enter did not apply high verbosity");
    assert!(a.pending.is_none());
    assert!(a.input.is_empty(), "the command line stayed: {:?}", a.input);
}

#[test]
fn backspace_from_a_value_picker_returns_to_the_command() {
    let mut a = app();
    let (tx, _rx) = channel();
    for c in "/model ".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    assert_eq!(open_picker(&a).kind, PickKind::Model);
    a.handle_key(Key::Backspace, &tx);
    let picker = open_picker(&a);
    assert_eq!(picker.kind, PickKind::Command);
    assert_eq!(picker.filter, "model");
    assert_eq!(a.input, "/model");
}

#[test]
fn free_text_arguments_keep_the_composer() {
    let mut a = app();
    let (tx, _rx) = channel();
    for c in "/stash remember the tests".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    assert!(a.pending.is_none(), "a stash draft opened a picker");
    assert_eq!(a.input, "/stash remember the tests");
}

#[test]
fn a_typed_cd_path_takes_over_from_the_browser() {
    let mut a = app();
    let (tx, _rx) = channel();
    for c in "/cd ".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    assert_eq!(open_picker(&a).kind, PickKind::Dir);
    a.handle_key(Key::Char('/'), &tx);
    assert!(a.pending.is_none(), "the browser kept the slash");
    assert_eq!(a.input, "/cd /");
}

#[test]
fn a_long_label_gives_way_to_the_right_column() {
    let items = ["short", &"a very long session title ".repeat(4)]
        .iter()
        .enumerate()
        .map(|(i, t)| PickItem {
            label: t.to_string(),
            value: i.to_string(),
            actions: Vec::new(),
            desc: format!("{} msgs", 10 + i * 50),
        })
        .collect();
    let picker = Picker::new("sessions", items, PickKind::Session);
    let short = picker_row(&picker, 0, 0, 50, false).remove(0);
    let long = picker_row(&picker, 1, 1, 50, false).remove(0);
    assert!(long.contains('…') && long.ends_with("60 msgs"), "{long}");
    assert!(long.chars().count() <= 50, "{long}");
    assert_eq!(short.find("10 msgs"), long.find("60 msgs"), "{short}\n{long}");
}

#[test]
fn command_rows_show_aligned_descriptions() {
    let picker = Picker::new("commands", command_items(), PickKind::Command);
    let row = |label: &str| {
        let at = picker
            .items
            .iter()
            .position(|item| item.label == label)
            .unwrap_or_else(|| panic!("no row {label}"));
        picker_row(&picker, at, at, 70, false).remove(0)
    };
    let compact = row("/compact");
    let help = row("/help");
    assert!(compact.contains("compact the conversation now"), "{compact}");
    assert!(help.contains("show the commands"), "{help}");
    let desc_col = |line: &str, desc: &str| {
        line[..line.find(desc).unwrap()].chars().count()
    };
    assert_eq!(
        desc_col(&compact, "compact the conversation now"),
        desc_col(&help, "show the commands"),
        "descriptions are not aligned:\n{compact}\n{help}"
    );
    assert!(help.contains("▸ /help"), "no marker: {help}");
}

#[test]
fn the_highlighted_command_description_wraps() {
    let mut picker = Picker::new("commands", command_items(), PickKind::Command);
    let at = picker
        .items
        .iter()
        .position(|i| i.label == "/goal")
        .expect("/goal");
    picker.index = at;
    picker.offset = at;
    let lines = picker_lines(&picker, &picker.matches(), 8, 46, true);
    let row: Vec<&String> = lines
        .iter()
        .filter(|(_, i, _)| *i == at)
        .map(|(l, _, _)| l)
        .collect();
    assert_eq!(row.len(), 2, "{row:?}");
    assert!(row[1].contains("off to stop"), "description cut: {:?}", row[1]);
    assert_eq!(
        row[1].chars().take_while(|c| *c == ' ').count(),
        16,
        "no hanging indent: {:?}",
        row[1]
    );
    // Rows that are not highlighted keep one line, so only the
    // highlighted item adds a row.
    assert_eq!(lines.len(), 9, "another row wrapped: {lines:?}");
}

#[test]
fn a_description_too_long_for_two_lines_ends_in_an_ellipsis() {
    let mut picker = Picker::new("commands", command_items(), PickKind::Command);
    let at = picker
        .items
        .iter()
        .position(|i| i.label == "/goal")
        .expect("/goal");
    picker.index = at;
    let row = picker_row(&picker, at, at, 38, true);
    assert_eq!(row.len(), 2, "{row:?}");
    assert!(row[1].ends_with('…'), "{:?}", row[1]);
    assert!(
        row.iter().all(|l| l.chars().count() <= 38),
        "a line is wider than the box: {row:?}"
    );
}

#[test]
fn the_command_picker_shows_the_whole_description() {
    let mut a = app();
    let (tx, _cmds) = channel();
    for c in "/goal".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    let frame = strip(&a.frame(60, 20));
    assert!(frame.contains("/goal"), "no command row: {frame:?}");
    assert!(frame.contains("off to stop"), "description cut: {frame:?}");
}

#[test]
fn a_short_screen_cuts_the_description_on_one_line() {
    let mut a = app();
    let mut picker = Picker::new("commands", command_items(), PickKind::Command);
    let at = picker
        .items
        .iter()
        .position(|i| i.label == "/goal")
        .expect("/goal");
    // Scrolled so the row is in the box, whatever comes before it.
    picker.index = at;
    picker.offset = at;
    a.pending = Some(Pending::Pick(picker));
    let frame = strip(&a.frame(60, 12));
    assert!(frame.contains("/goal"), "no command row: {frame:?}");
    assert!(!frame.contains("off to stop"), "the box grew: {frame:?}");
}

#[test]
fn a_wrapped_line_carries_the_same_click_target() {
    let mut a = app();
    let (tx, _cmds) = channel();
    for c in "/goal".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    a.frame(60, 20);
    let hits: Vec<usize> = a
        .targets
        .iter()
        .filter_map(|t| match t.3 {
            Target::Row(0) => Some(t.0),
            _ => None,
        })
        .collect();
    assert_eq!(hits.len(), 2, "the second line has no target: {:?}", a.targets);
    assert_eq!(hits[1], hits[0] + 1, "{hits:?}");
}

#[test]
fn a_missing_key_opens_the_provider_picker() {
    let mut a = app();
    a.begin_onboard("DEEPSEEK_API_KEY".into());
    assert!(a.onboard);
    assert_eq!(open_picker(&a).kind, PickKind::Provider);
    let frame = strip(&a.frame(80, 20));
    assert!(!frame.contains("no key for"), "the missing-key line is shown: {frame}");
    assert!(!frame.contains("choose a provider"), "the missing-key line is shown: {frame}");
    assert!(frame.contains("provider"), "the provider setup is not shown: {frame}");
}

#[test]
fn a_provider_without_a_key_starts_at_once() {
    let mut a = app();
    let (tx, _cmds) = channel();
    a.begin_onboard("DEEPSEEK_API_KEY".into());
    a.choose_onboard_provider("ollama", &tx);
    assert!(a.pending.is_none(), "a key row opened for Ollama");
    assert!(!a.onboard);
    assert_eq!(a.cfg.provider, crate::providers::Provider::Ollama);
}

#[test]
fn the_first_run_key_is_masked_and_editable() {
    let mut a = app();
    let (tx, _cmds) = channel();
    a.begin_onboard("DEEPSEEK_API_KEY".into());
    a.choose_onboard_provider("deepseek", &tx);
    assert!(matches!(a.pending, Some(Pending::KeyEntry { .. })), "no key row");
    for c in "sk-ab".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    let row = |a: &mut App| {
        strip(&a.frame(80, 20))
            .lines()
            .find(|l| l.contains("key for DEEPSEEK_API_KEY"))
            .map(str::to_string)
            .expect("the key row")
    };
    let shown = row(&mut a);
    let stars = shown.matches('*').count();
    assert_eq!(stars, 5, "{shown}");
    assert!(!shown.contains("sk-ab"), "the key is drawn: {shown}");
    a.handle_key(Key::Backspace, &tx);
    assert_eq!(row(&mut a).matches('*').count(), 4, "Backspace did not remove");
}

#[test]
fn esc_leaves_the_first_run() {
    let mut a = app();
    let (tx, _cmds) = channel();
    a.begin_onboard("DEEPSEEK_API_KEY".into());
    a.choose_onboard_provider("deepseek", &tx);
    a.handle_key(Key::Esc, &tx);
    assert!(a.quit, "Esc kept the first run open");
    assert!(a.pending.is_none());
}

#[test]
fn a_refused_key_retries_and_a_dead_network_saves() {
    assert_eq!(first_run_outcome(Ok(())), KeyOutcome::Save(None));
    assert_eq!(
        first_run_outcome(Err("models: HTTP 401".into())),
        KeyOutcome::Retry("models: HTTP 401".into())
    );
    match first_run_outcome(Err("models: connection refused".into())) {
        KeyOutcome::Save(Some(warning)) => {
            assert!(warning.contains("check did not run"), "{warning}")
        }
        other => panic!("offline did not save: {other:?}"),
    }
}

#[test]
fn the_mode_picker_lists_allows_below_the_modes() {
    let mut a = app();
    let (tx, _cmds) = channel();
    a.allows = vec![("npm test".into(), false), ("cargo test".into(), true)];
    a.run_command("mode", &tx);
    let picker = open_picker(&a);
    assert_eq!(picker.items.len(), 7, "modes, allows, and the add row");
    assert_eq!(picker.items[4].label, "npm test");
    assert_eq!(picker.items[4].desc, "session");
    assert_eq!(picker.items[5].desc, "always");
    assert_eq!(picker.items[4].value, "allow:npm test");
}

#[test]
fn the_mode_picker_with_no_allows_shows_only_the_modes() {
    let mut a = app();
    let (tx, _cmds) = channel();
    a.run_command("mode", &tx);
    assert_eq!(open_picker(&a).items.len(), 5, "four modes and the add row");
}

#[test]
fn the_mode_picker_adds_a_rule() {
    let (mut a, cmds, _ev) = worker_app();
    let tx = a.worker.as_ref().unwrap().0.clone();
    a.run_command("mode", &tx);
    let at = open_picker(&a)
        .items
        .iter()
        .position(|i| i.value == "add-rule")
        .expect("the add row");
    if let Some(Pending::Pick(p)) = &mut a.pending {
        p.index = at;
    }
    a.handle_key(Key::Enter, &tx);
    assert!(matches!(a.pending, Some(Pending::AllowRule { .. })), "no rule prompt");
    for c in "cargo test".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    a.handle_key(Key::Enter, &tx);
    assert!(matches!(cmds.try_recv(), Ok(Cmd::AllowAlways(s)) if s == "cargo test"));
    a.apply(Ev::Allow { shape: "cargo test".into(), always: true });
    assert!(a.allows.contains(&("cargo test".to_string(), true)), "not listed");
    a.run_command("mode", &tx);
    let row = open_picker(&a)
        .items
        .iter()
        .find(|i| i.label == "cargo test")
        .cloned()
        .expect("the saved rule");
    assert_eq!(row.desc, "always");
}

#[test]
fn an_empty_rule_stays_open_and_saves_nothing() {
    let (mut a, cmds, _ev) = worker_app();
    let tx = a.worker.as_ref().unwrap().0.clone();
    a.pending = Some(Pending::AllowRule { value: String::new() });
    a.handle_key(Key::Enter, &tx);
    assert!(matches!(a.pending, Some(Pending::AllowRule { .. })), "the prompt closed");
    assert!(a.allows.is_empty());
    assert!(cmds.try_recv().is_err(), "an empty rule reached the worker");
}

#[test]
fn a_declined_shape_is_listed_and_one_key_allows_it() {
    let (mut a, cmds, _ev) = worker_app();
    let tx = a.worker.as_ref().unwrap().0.clone();
    a.apply(Ev::ShapeAsked { shape: "npm test".into(), allowed: false });
    a.run_command("mode", &tx);
    let at = open_picker(&a)
        .items
        .iter()
        .position(|i| i.label == "npm test")
        .expect("the ask row");
    assert_eq!(open_picker(&a).items[at].desc, "asked · declined");
    if let Some(Pending::Pick(p)) = &mut a.pending {
        p.index = at;
    }
    a.handle_key(Key::Char('a'), &tx);
    assert!(matches!(cmds.try_recv(), Ok(Cmd::AllowAlways(s)) if s == "npm test"));
    a.apply(Ev::Allow { shape: "npm test".into(), always: true });
    assert!(a.allows.contains(&("npm test".to_string(), true)), "not allowed");
    a.run_command("mode", &tx);
    let picker = open_picker(&a);
    assert_eq!(
        picker.items.iter().filter(|i| i.label == "npm test").count(),
        1,
        "listed as both an allow and an ask"
    );
}

#[test]
fn at_most_five_asks_are_listed_newest_first() {
    let mut a = app();
    let (tx, _cmds) = channel();
    for n in 0..7 {
        a.apply(Ev::ShapeAsked { shape: format!("cmd {n}"), allowed: n % 2 == 0 });
    }
    a.run_command("mode", &tx);
    let asks: Vec<String> = open_picker(&a)
        .items
        .iter()
        .filter(|i| i.value.starts_with("asked:"))
        .map(|i| i.label.clone())
        .collect();
    assert_eq!(asks, ["cmd 6", "cmd 5", "cmd 4", "cmd 3", "cmd 2"]);
}

#[test]
fn the_mode_footer_offers_revoke_on_an_allow_row() {
    let mut a = app();
    let (tx, _cmds) = channel();
    a.allows = vec![("npm test".into(), false)];
    a.run_command("mode", &tx);
    if let Some(Pending::Pick(p)) = &mut a.pending {
        p.index = 4;
    }
    let frame = strip(&a.frame(100, 30));
    assert!(frame.contains("d revoke"), "no revoke hint: {frame:?}");
}

#[test]
fn d_revokes_the_highlighted_allow() {
    let mut a = app();
    let (tx, rx) = channel();
    a.allows = vec![("npm test".into(), false), ("cargo test".into(), true)];
    a.run_command("mode", &tx);
    if let Some(Pending::Pick(p)) = &mut a.pending {
        p.index = 4;
    }
    a.handle_key(Key::Char('d'), &tx);
    assert_eq!(a.allows, vec![("cargo test".to_string(), true)]);
    match rx.try_recv() {
        Ok(Cmd::RevokeAllow { shape, always }) => {
            assert_eq!(shape, "npm test");
            assert!(!always, "a session allow was revoked as always");
        }
        _ => panic!("no revoke command"),
    }
    let picker = open_picker(&a);
    assert_eq!(picker.items.len(), 6, "the row stayed in the list");
    assert!(picker.items.iter().all(|i| i.label != "npm test"));
}

#[test]
fn choosing_an_allow_row_changes_nothing() {
    let mut a = app();
    let (tx, _cmds) = channel();
    a.allows = vec![("npm test".into(), false)];
    a.run_command("mode", &tx);
    if let Some(Pending::Pick(p)) = &mut a.pending {
        p.index = 4;
    }
    a.handle_key(Key::Enter, &tx);
    assert!(matches!(a.pending, Some(Pending::Pick(_))), "the picker closed");
    assert_eq!(a.cfg.live_permissions(), crate::config::Permissions::Normal);
}

#[test]
fn the_picker_shows_its_filter_cursor() {
    let mut a = app();
    let (tx, _rx) = channel();
    let idle = a.frame(80, 20);
    assert!(!idle.contains("\x1b[?25h"), "idle frame showed a cursor");
    a.handle_key(Key::Char('/'), &tx);
    let open = a.frame(80, 20);
    assert!(open.contains("\x1b[?25h"), "the filter has no cursor");
    a.handle_key(Key::Esc, &tx);
    let closed = a.frame(80, 20);
    assert!(!closed.contains("\x1b[?25h"), "the cursor stayed after closing");
}

#[test]
fn the_directory_browser_stops_at_the_filesystem_root() {
    let root = scratch("fsroot");
    let mut picker = Picker::browse("cd", PickKind::Dir, &root);
    picker.current = std::path::PathBuf::from("/");
    assert!(!picker.parent(), "Left moved above /");
    assert_eq!(picker.current, std::path::PathBuf::from("/"));
    picker.reload();
    assert!(picker.items.iter().all(|i| i.label != "../"), "/ has a parent row");
}

#[test]
fn browsers_name_their_directory() {
    let base = scratch("title");
    let root = base.join("work");
    std::fs::create_dir_all(root.join("src")).unwrap();
    let mut a = app_at(root.clone());
    let (tx, _rx) = channel();
    a.run_command("cd", &tx);
    highlight(&mut a, &tx, "src/");
    a.handle_key(Key::Tab, &tx);
    let frame = strip(&a.frame(200, 30));
    let start = frame.find("╭─ cd ").expect("title");
    let top = &frame[start..start + frame[start..].find('╮').unwrap()];
    assert!(top.contains("/work/src "), "{top}");
    assert!(top.chars().count() <= 72, "{top}");
    assert_eq!(tilde(std::path::Path::new(&std::env::var("HOME").unwrap())), "~");
}

#[test]
fn the_listing_starts_with_this_folder_and_its_parent() {
    let base = scratch("up");
    let root = base.join("work");
    std::fs::create_dir_all(root.join("leaf")).unwrap();
    let mut a = app_at(root.clone());
    let (tx, rx) = channel();
    a.run_command("cd", &tx);
    let labels: Vec<String> = open_picker(&a).items.iter().map(|i| i.label.clone()).collect();
    assert_eq!(labels, vec!["./", "../", "leaf/"]);
    assert!(open_picker(&a).footer().starts_with("Enter choose · Tab/→ open · ← up"));

    // Tab on `../` opens the parent; Tab on `./` stays put.
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Tab, &tx);
    let above = base.canonicalize().unwrap();
    assert_eq!(open_picker(&a).current, above, "Tab did not open the parent");
    a.handle_key(Key::Tab, &tx);
    assert_eq!(open_picker(&a).current, above, "Tab on ./ moved");

    // A leaf folder: open it, then Enter on `./` chooses it.
    highlight(&mut a, &tx, "work/");
    a.handle_key(Key::Right, &tx);
    highlight(&mut a, &tx, "leaf/");
    a.handle_key(Key::Right, &tx);
    let labels: Vec<String> = open_picker(&a).items.iter().map(|i| i.label.clone()).collect();
    assert_eq!(labels, vec!["./", "../"]);
    a.handle_key(Key::Enter, &tx);
    match rx.try_recv() {
        Ok(Cmd::SetRoot(path)) => assert_eq!(path, root.join("leaf")),
        _ => panic!("Enter on ./ did not choose the folder"),
    }
    assert!(a.pending.is_none());

    a.run_command("image", &tx);
    assert!(open_picker(&a).items.iter().all(|i| i.label != "../" && i.label != "./"));
}

#[test]
fn cwd_commands_report_and_change_the_root() {
    let mut a = app();
    let (tx, rx) = channel();
    assert_eq!(a.root, std::env::temp_dir(), "constructor root ignored");

    a.run_command("cwd", &tx);
    assert_eq!(a.entries.last().unwrap().text, a.root.display().to_string());

    a.run_command("cd", &tx);
    assert_eq!(
        open_picker(&a).kind,
        PickKind::Dir,
        "bare cd opened no browser"
    );
    let (tx, rx) = channel();
    a.run_command("cd", &tx);
    a.handle_key(Key::Esc, &tx);
    assert!(a.pending.is_none(), "Esc left the browser open");
    assert!(rx.try_recv().is_err(), "Esc changed the root");

    let before = a.entries.len();
    a.run_command("cd .", &tx);
    match rx.try_recv() {
        Ok(Cmd::SetRoot(path)) => {
            assert!(path.is_dir());
            // The worker's single report is the only cwd line.
            a.apply(Ev::Info(root_report(&path)));
        }
        _ => panic!("cd did not send SetRoot"),
    }
    let lines: Vec<&str> = a.entries[before..]
        .iter()
        .filter(|e| e.text.starts_with("cwd:"))
        .map(|e| e.text.as_str())
        .collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].ends_with("(prompt cache reset)"), "{lines:?}");
    let home = std::path::PathBuf::from(std::env::var("HOME").unwrap()).join("Projects");
    assert_eq!(root_report(&home), "cwd: ~/Projects (prompt cache reset)");

    let dir = std::env::temp_dir();
    a.apply(Ev::Root(dir.clone()));
    assert_eq!(a.root, dir);
    a.run_local("pwd");
    let shown = dir.canonicalize().unwrap().display().to_string();
    assert!(
        a.entries.last().unwrap().text.contains(&shown),
        "typed shell did not follow the root: {:?}",
        a.entries.last().unwrap().text
    );

    a.run_command("cd /nope/imp-missing", &tx);
    assert_eq!(a.entries.last().unwrap().role, Role::Error);
}

fn highlight(a: &mut App, tx: &Sender<Cmd>, label: &str) {
    let at = {
        let picker = open_picker(a);
        picker
            .matches()
            .iter()
            .position(|&i| picker.items[i].label == label)
            .unwrap_or_else(|| panic!("no row {label:?}"))
    };
    for _ in 0..at {
        a.handle_key(Key::Down, tx);
    }
}

#[test]
fn directory_browser_navigates_selects_and_climbs() {
    let base = scratch("dir");
    let root = base.join("work");
    std::fs::create_dir_all(root.join("src").join("nested")).unwrap();
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::create_dir_all(base.join("sibling")).unwrap();
    std::fs::write(root.join("notes.txt"), "x").unwrap();

    let mut a = app_at(root.clone());
    let (tx, rx) = channel();
    a.run_command("cd", &tx);
    {
        let picker = open_picker(&a);
        let labels: Vec<&str> = picker.items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"src/"), "{labels:?}");
        assert!(labels.contains(&"docs/"), "{labels:?}");
        assert!(!labels.contains(&"notes.txt"), "a file was listed");
    }

    highlight(&mut a, &tx, "src/");
    a.handle_key(Key::Tab, &tx);
    {
        let picker = open_picker(&a);
        assert_eq!(picker.current, root.join("src"), "Tab did not open");
        assert!(picker.items.iter().any(|i| i.label == "nested/"));
        assert!(!picker.items.iter().any(|i| i.label == "docs/"));
    }
    assert_eq!(a.root, root, "opening changed the session root");

    a.handle_key(Key::Left, &tx);
    assert_eq!(open_picker(&a).current, root, "Left did not go up");
    a.handle_key(Key::Left, &tx);
    let above = base.canonicalize().unwrap();
    assert_eq!(open_picker(&a).current, above, "Left did not climb");
    assert!(rx.try_recv().is_err(), "navigating changed the root");

    let back = format!("{}/", root.file_name().unwrap().to_string_lossy());
    highlight(&mut a, &tx, &back);
    a.handle_key(Key::Right, &tx);
    assert_eq!(open_picker(&a).current, root, "Right did not open");

    highlight(&mut a, &tx, "src/");
    a.handle_key(Key::Enter, &tx);
    match rx.try_recv() {
        Ok(Cmd::SetRoot(path)) => assert_eq!(path, root.join("src")),
        _ => panic!("Enter did not choose the directory"),
    }
    assert!(a.pending.is_none(), "Enter left the browser open");
    assert_eq!(a.root, root, "the root moved before the worker replied");
}

#[test]
fn the_image_browser_opens_folders_with_right() {
    let root = scratch("img-right");
    std::fs::create_dir_all(root.join("shots")).unwrap();
    std::fs::write(root.join("shots").join("a.png"), b"\x89PNG\r\n\x1a\n").unwrap();
    let mut a = app_at(root.clone());
    let (tx, _rx) = channel();
    a.run_command("image", &tx);
    assert!(open_picker(&a).footer().starts_with("Enter attach · Tab/→ open"));
    highlight(&mut a, &tx, "shots/");
    a.handle_key(Key::Right, &tx);
    let picker = open_picker(&a);
    assert_eq!(picker.current, root.join("shots"), "Right did not open");
    assert!(picker.items.iter().any(|i| i.label == "a.png"));
}

#[test]
fn image_browser_lists_images_and_attaches_on_enter() {
    let root = scratch("img");
    std::fs::create_dir_all(root.join("Pictures")).unwrap();
    std::fs::create_dir_all(root.join("docs")).unwrap();
    let png = b"\x89PNG\r\n\x1a\n1234567890";
    std::fs::write(root.join("Pictures").join("shot.png"), png).unwrap();
    std::fs::write(root.join("Pictures").join("notes.txt"), "text").unwrap();

    let mut a = app_at(root.clone());
    let (tx, rx) = channel();
    a.run_command("image", &tx);
    {
        let picker = open_picker(&a);
        assert_eq!(picker.kind, PickKind::Image);
        let labels: Vec<&str> = picker.items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"Pictures/"), "{labels:?}");
        assert!(labels.contains(&"docs/"), "{labels:?}");
        assert!(!labels.contains(&"notes.txt"), "a text file was listed");
    }

    highlight(&mut a, &tx, "Pictures/");
    a.handle_key(Key::Enter, &tx);
    assert_eq!(open_picker(&a).current, root.join("Pictures"));
    assert!(rx.try_recv().is_err(), "Enter on a directory attached");

    {
        let picker = open_picker(&a);
        let labels: Vec<&str> = picker.items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"shot.png"), "{labels:?}");
        assert!(!labels.contains(&"notes.txt"), "a text file was listed");
    }
    highlight(&mut a, &tx, "shot.png");
    a.handle_key(Key::Enter, &tx);
    assert!(a.pending.is_none(), "the image picker stayed open");
    match rx.try_recv() {
        Ok(Cmd::Attach(images)) => {
            assert_eq!(images.len(), 1);
            assert_eq!(images[0].name, "shot.png");
        }
        _ => panic!("Enter did not attach the image"),
    }
    assert!(
        a.entries
            .last()
            .map(|e| e.text.contains("shot.png"))
            .unwrap_or(false),
        "no attachment marker"
    );
}

#[test]
fn agent_command_switches_by_name_and_reports_current() {
    let mut a = app();
    let (tx, rx) = channel();
    a.run_command("agent reviewer", &tx);
    match rx.try_recv() {
        Ok(Cmd::SetAgent(name)) => assert_eq!(name, "reviewer"),
        _ => panic!("no direct switch"),
    }
    a.run_command("agent current", &tx);
    assert!(matches!(rx.try_recv(), Ok(Cmd::ShowAgent)));
}

#[test]
fn fork_command_sends_the_fork() {
    let mut a = app();
    let (tx, rx) = channel();
    a.run_command("fork", &tx);
    assert!(matches!(rx.try_recv(), Ok(Cmd::Fork)));
}

#[test]
fn slash_commands_with_values_reach_the_parser() {
    let mut a = app();
    let (tx, rx) = channel();
    a.input = "/cd .".into();
    a.cursor = a.input.len();
    a.submit(&tx, false);
    assert!(matches!(rx.try_recv(), Ok(Cmd::SetRoot(_))));
}

#[test]
fn verbosity_command_controls_tool_detail() {
    let mut a = app();
    let (tx, _cmds) = channel();
    assert_eq!(a.verbosity, Verbosity::Medium, "default is not medium");
    let row = |a: &mut App| {
        a.apply(Ev::ToolStart { name: "bash".into(), summary: "run `ls`".into(), args: "{}".into() });
        a.apply(Ev::ToolEnd {
            ok: true,
            preview: "exit 0".into(),
            output: "exit 0\nfoo".into(),
            detail: String::new(),
            ms: 4,
        });
        a.entries.last().unwrap().text.clone()
    };
    let medium = row(&mut a);
    assert!(medium.starts_with("✓ run `ls` → exit 0"), "{medium:?}");
    a.run_command("verbosity low", &tx);
    let low = row(&mut a);
    assert!(low.starts_with("✓ run `ls`") && !low.contains('→'), "{low:?}");
    a.entries.clear();
    a.run_command("verbosity high", &tx);
    assert_eq!(a.verbosity, Verbosity::High);

    a.apply(Ev::ToolStart {
        name: "bash".into(),
        summary: "run `ls`".into(),
        args: "{\"command\":\"ls\"}".into(),
    });
    a.apply(Ev::ToolEnd {
        ok: true,
        preview: "total 8".into(),
        output: "total 8\nfoo".into(),
        detail: String::new(),
        ms: 4,
    });
    // The verbosity confirmation is a toast, so the tool row comes first.
    let text = &a.entries[0].text;
    assert!(text.contains("\n  $ ls"), "{text:?}");
    assert!(text.contains("foo"), "{text:?}");

    a.run_command("verbosity low", &tx);
    assert_eq!(a.verbosity, Verbosity::Low);
    a.run_command("verbosity", &tx);
    let picker = open_picker(&a);
    assert_eq!(
        picker.kind,
        PickKind::Verbosity,
        "bare verbosity opened no picker"
    );
    assert_eq!(
        picker.selected().map(|i| i.value.as_str()),
        Some("low"),
        "the current level is not highlighted"
    );
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Enter, &tx);
    assert_eq!(a.verbosity, Verbosity::Medium, "choosing medium did nothing");
    assert!(a.pending.is_none(), "the picker stayed open");
    a.run_command("verbosity loud", &tx);
    assert!(a.entries.last().unwrap().text.contains("use low, medium, or high"));
}

#[test]
fn agent_picker_enter_switches_and_esc_does_not() {
    let mut a = app();
    open_agent_picker(&mut a);
    let (tx, rx) = channel();
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Enter, &tx);
    match rx.try_recv() {
        Ok(Cmd::SetAgent(name)) => assert_eq!(name, "reviewer"),
        _ => panic!("the agent picker sent no switch"),
    }
    assert!(a.pending.is_none(), "the picker stayed open");
    assert_eq!(a.agent, "reviewer");

    open_agent_picker(&mut a);
    a.handle_key(Key::Esc, &tx);
    assert!(a.pending.is_none());
    assert!(rx.try_recv().is_err(), "Esc sent a command");
    assert_eq!(a.agent, "reviewer", "Esc changed the agent");
}

#[test]
fn session_picker_esc_sends_nothing() {
    let mut a = picker();
    let (tx, rx) = channel();
    a.handle_key(Key::Esc, &tx);
    assert!(a.pending.is_none(), "the picker stayed open");
    assert!(rx.try_recv().is_err(), "Esc resumed anyway");
}

#[test]
fn a_failed_call_puts_its_preview_on_its_own_line() {
    let mut a = app();
    a.apply(Ev::ToolStart {
        name: "bash".into(),
        summary: "run `openspec list`".into(),
        args: "{\"command\":\"openspec list\"}".into(),
    });
    a.apply(Ev::ToolEnd {
        ok: false,
        preview: "Changes: render-on-change".into(),
        output: "Changes: render-on-change".into(),
        detail: String::new(),
        ms: 7,
    });
    let text = &a.entries[0].text;
    assert!(text.contains("\nChanges: render-on-change"), "{text:?}");
    assert!(!text.contains(" · Changes:"), "{text:?}");
    let out = a.frame(60, 12);
    assert!(out.contains("  Changes: render-on-change"), "{out:?}");
}

#[test]
fn tool_marks_are_bold() {
    let mut a = colour_app();
    a.apply(Ev::ToolStart {
        name: "bash".into(),
        summary: "run `true`".into(),
        args: "{}".into(),
    });
    a.apply(Ev::ToolEnd {
        ok: true,
        preview: String::new(),
        output: String::new(),
        detail: String::new(),
        ms: 1,
    });
    let out = a.frame(80, 12);
    let success = a.palette.paint(Role::Success, "✓");
    assert!(out.contains(&format!("\x1b[1m{success}")), "mark not bold success: {out:?}");
    let tool = a.palette.paint(Role::Tool, " run `true`");
    assert!(out.contains(&tool), "summary left the tool colour: {out:?}");

    a.palette = Palette {
        on: false,
        allowed: false,
        truecolor: false,
        theme: &crate::theme::THEMES[0],
    };
    let plain = a.frame(80, 12);
    assert!(
        !plain.contains("\x1b[1m"),
        "bold with colour off: {plain:?}"
    );
}

#[test]
fn a_wrapped_tool_row_keeps_its_indent() {
    let mut a = app();
    a.apply(Ev::ToolStart {
        name: "bash".into(),
        summary: format!("run `echo {}`", "a".repeat(60)),
        args: "{}".into(),
    });
    let out = a.frame(30, 10);
    let mut seen = 0;
    for line in out.split("\r\n") {
        if line.contains("aaaa") {
            let body = line.trim_start_matches("\x1b[2K");
            assert!(body.starts_with("  "), "lost indent: {body:?}");
            seen += 1;
        }
    }
    assert!(seen > 1, "summary did not wrap: {out:?}");
}

fn tool_call(a: &mut App, summary: String) {
    a.apply(Ev::ToolStart {
        name: "bash".into(),
        summary,
        args: "{}".into(),
    });
    a.apply(Ev::ToolEnd {
        ok: true,
        preview: String::new(),
        output: String::new(),
        detail: String::new(),
        ms: 4,
    });
}

#[test]
fn tool_rows_start_in_the_same_column() {
    let mut a = app();
    a.apply(Ev::ToolStart {
        name: "bash".into(),
        summary: "run `ls`".into(),
        args: "{}".into(),
    });
    a.apply(Ev::ToolEnd {
        ok: true,
        preview: String::new(),
        output: String::new(),
        detail: String::new(),
        ms: 4,
    });
    a.apply(Ev::ToolStart {
        name: "bash".into(),
        summary: "run `false`".into(),
        args: "{}".into(),
    });
    a.apply(Ev::ToolEnd {
        ok: false,
        preview: "exit 1".into(),
        output: "exit 1".into(),
        detail: String::new(),
        ms: 7,
    });
    let painted = strip(&a.frame(60, 12));
    for row in painted.split("\r\n") {
        if row.contains('✓') || row.contains('✗') {
            assert!(
                row.starts_with('✓') || row.starts_with('✗'),
                "tool row is indented: {row:?}"
            );
        }
    }
}

#[test]
fn a_long_tool_summary_wraps_and_keeps_its_duration() {
    let mut a = app();
    let command = format!("echo {}", "a".repeat(300));
    a.apply(Ev::ToolStart {
        name: "bash".into(),
        summary: crate::agent::describe("bash", &serde_json::json!({"command": command})),
        args: "{}".into(),
    });
    a.apply(Ev::ToolEnd {
        ok: true,
        preview: String::new(),
        output: String::new(),
        detail: String::new(),
        ms: 12,
    });
    let painted = strip(&a.frame(80, 30));
    let rows: Vec<&str> = painted
        .split("\r\n")
        .filter(|row| row.contains("aaaa"))
        .collect();
    assert!(rows.len() > 1, "the summary did not wrap: {painted:?}");
    let last = rows.last().unwrap();
    assert!(last.ends_with("12ms"), "the duration left the row end: {painted:?}");
    assert_eq!(last.chars().count(), 80, "{last:?}");
    assert!(!rows.iter().any(|r| r.contains('…')), "the summary was cut: {painted:?}");
}

#[test]
fn durations_sit_at_the_right_edge() {
    let finish = |summary: &str, ms: u128| {
        let mut a = app();
        a.apply(Ev::ToolStart {
            name: "bash".into(),
            summary: summary.into(),
            args: "{}".into(),
        });
        a.apply(Ev::ToolEnd {
            ok: true,
            preview: String::new(),
            output: String::new(),
            detail: String::new(),
            ms,
        });
        strip(&a.frame(40, 12))
    };
    let fast = finish("run `ls`", 5);
    let row = fast.split("\r\n").find(|r| r.contains("run `ls`")).unwrap();
    assert!(row.starts_with("✓ run `ls`") && row.ends_with("5ms"), "{row:?}");
    assert_eq!(row.chars().count(), 40);
    let slow = finish("run `ls`", 1400);
    assert!(slow.contains("1.4s"), "{slow:?}");
    // 38 columns of summary leave no room: the time takes the next row.
    let full = finish(&"x".repeat(36), 7);
    let rows: Vec<&str> = full.split("\r\n").collect();
    let at = rows.iter().position(|r| r.contains("xxxx")).unwrap();
    assert!(!rows[at].contains("7ms"), "{rows:?}");
    assert!(rows[at + 1].trim() == "7ms" && rows[at + 1].ends_with("7ms"), "{rows:?}");
}

#[test]
fn a_multi_line_summary_keeps_its_duration_last() {
    let mut a = app();
    a.apply(Ev::ToolStart {
        name: "bash".into(),
        summary: "run `cat <<EOF\nhello\nEOF`".into(),
        args: "{}".into(),
    });
    a.apply(Ev::ToolEnd {
        ok: true,
        preview: String::new(),
        output: String::new(),
        detail: String::new(),
        ms: 9,
    });
    let frame = strip(&a.frame(60, 12));
    let rows: Vec<&str> = frame.split("\r\n").collect();
    let first = rows.iter().position(|r| r.contains("cat <<EOF")).unwrap();
    assert!(!rows[first].contains("9ms"), "{rows:?}");
    let last = rows.iter().position(|r| r.contains("EOF`")).unwrap();
    assert!(rows[last].ends_with("9ms"), "{rows:?}");
}

#[test]
fn a_long_failure_preview_wraps_under_the_indent() {
    let mut a = app();
    a.apply(Ev::ToolStart {
        name: "bash".into(),
        summary: "run `false`".into(),
        args: "{}".into(),
    });
    a.apply(Ev::ToolEnd {
        ok: false,
        preview: "exit 1 timed out after 30s ".to_string() + &"reason ".repeat(20),
        output: String::new(),
        detail: String::new(),
        ms: 30_000,
    });
    let painted = strip(&a.frame(70, 20));
    let rows: Vec<&str> = painted
        .split("\r\n")
        .filter(|row| row.contains("reason"))
        .collect();
    assert!(rows.len() > 1, "the failure preview did not wrap: {painted:?}");
    for row in rows {
        assert!(row.starts_with("  "), "preview lost its indent: {row:?}");
    }
}

#[test]
fn an_ask_question_wraps_instead_of_clipping() {
    let mut a = app();
    let question = "delete all five files? ".repeat(12);
    a.apply(Ev::ToolStart {
        name: "ask".into(),
        summary: crate::agent::describe("ask", &serde_json::json!({"question": question})),
        args: "{}".into(),
    });
    let painted = strip(&a.frame(80, 30));
    let rows: Vec<&str> = painted
        .split("\r\n")
        .filter(|row| row.contains("delete all five files?"))
        .collect();
    assert!(rows.len() > 1, "the question did not wrap: {painted:?}");
}

#[test]
fn failed_tools_render_in_the_error_role() {
    let mut a = app();
    a.apply(Ev::ToolStart {
        name: "bash".into(),
        summary: "run `false`".into(),
        args: "{\"command\":\"false\"}".into(),
    });
    assert_eq!(a.entries.len(), 1, "a call should start as one row");
    assert!(
        a.entries[0].text.starts_with(spinner_frame(a.spinner)),
        "no spinner while running"
    );
    a.apply(Ev::ToolEnd {
        ok: false,
        preview: "exit 1".into(),
        output: "exit 1".into(),
        detail: String::new(),
        ms: 7,
    });
    assert_eq!(a.entries.len(), 1, "the finish added a second row");
    // A command that ran and exited 1 is a result, not a breakage.
    assert_eq!(a.entries[0].role, Role::Tool);
    assert!(a.entries[0].text.starts_with("! "), "{}", a.entries[0].text);
    assert!(a.entries[0].text.contains("run `false`"));
    assert!(a.entries[0].text.contains("ms"));
}

#[test]
fn ctrl_e_toggles_every_row_of_the_latest_turn() {
    let mut a = app();
    let (tx, _cmds) = channel();
    let step = |a: &mut App, cmd: &str, ok: bool, output: &str| {
        a.apply(Ev::ToolStart { name: "bash".into(), summary: format!("run `{cmd}`"), args: format!("{{\"command\":\"{cmd}\"}}") });
        a.apply(Ev::ToolEnd { ok, preview: String::new(), output: output.into(), detail: String::new(), ms: 1 });
    };
    a.begin_task();
    step(&mut a, "false", false, "refused: outside the session");
    step(&mut a, "true", true, "exit 0");
    a.handle_key(Key::Ctrl('e'), &tx);
    let expanded = a.entries.iter().filter(|e| e.text.contains("\n  $ ")).count();
    assert_eq!(expanded, 2, "both rows should expand");
    a.handle_key(Key::Ctrl('e'), &tx);
    assert!(!a.entries.iter().any(|e| e.text.contains("\n  $ ")), "they should collapse");
    // A new turn leaves the old rows behind.
    a.begin_task();
    step(&mut a, "ls", true, "exit 0");
    a.handle_key(Key::Ctrl('e'), &tx);
    assert_eq!(a.entries.iter().filter(|e| e.text.contains("\n  $ ls")).count(), 1);
    // Clearing the screen drops the rows, so Ctrl+E edits nothing stale.
    a.handle_key(Key::Ctrl('l'), &tx);
    a.push(Role::Assistant, "fresh");
    a.handle_key(Key::Ctrl('e'), &tx);
    assert_eq!(a.entries[0].text, "fresh");
}

#[test]
fn expanded_arguments_read_as_a_person_would() {
    assert_eq!(readable_args(r#"{"command":"python3 calc.py","description":"Run calc.py"}"#), vec!["$ python3 calc.py"]);
    assert_eq!(
        readable_args(r#"{"path":"calc.py","old":"a - b","new":"a + b","all":false}"#),
        vec!["path: calc.py", "all: false", "new: a + b", "old: a - b"]
    );
    assert_eq!(readable_args("not json"), vec!["not json"]);
    let mut a = app();
    let (tx, _cmds) = channel();
    a.apply(Ev::ToolStart { name: "bash".into(), summary: "Run calc.py".into(), args: r#"{"command":"python3 calc.py"}"#.into() });
    a.apply(Ev::ToolEnd { ok: true, preview: String::new(), output: "exit 0\n5".into(), detail: String::new(), ms: 1 });
    a.handle_key(Key::Ctrl('e'), &tx);
    let out = strip(&a.frame(60, 10));
    assert!(out.contains("    $ python3 calc.py") && !out.contains("{\""), "{out:?}");
    // A long argument is cut like long output.
    let body: String = (1..=50).map(|i| format!("line {i}\n")).collect();
    let args = serde_json::json!({"path": "big.txt", "content": body}).to_string();
    a.apply(Ev::ToolStart { name: "write".into(), summary: "write big.txt".into(), args });
    a.apply(Ev::ToolEnd { ok: true, preview: String::new(), output: "wrote".into(), detail: String::new(), ms: 1 });
    a.handle_key(Key::Ctrl('e'), &tx);
    let text = &a.entries.last().unwrap().text;
    assert!(text.contains("line 19") && !text.contains("line 20\n") && text.contains("… +31 lines"), "{text}");
}

#[test]
fn a_long_directory_shortens_in_steps() {
    let p = "~/Projects/imp/src/providers";
    assert_eq!(fit_path(p, 40).as_deref(), Some(p));
    assert_eq!(fit_path(p, 20).as_deref(), Some("~/P/i/src/providers"));
    assert_eq!(fit_path(p, 12).as_deref(), Some("…/providers"));
    assert_eq!(fit_path(p, 9).as_deref(), Some("providers"));
    assert_eq!(fit_path(p, 8).as_deref(), Some("…oviders"));
    assert_eq!(fit_path(p, 7), None, "below the floor it is left out");
    assert_eq!(fit_path("~/.config/imp/agents", 16).as_deref(), Some("~/.c/imp/agents"));
    assert_eq!(fit_path("/private/tmp/claude/demo", 16).as_deref(), Some("/p/t/claude/demo"));
    assert_eq!(fit_path("~/averyveryverylongname", 10).as_deref(), Some("…ylongname"));
}

#[test]
fn diff_bands_reach_the_edge() {
    let mut a = colour_app();
    a.push_diffs(vec!["+short line".into()], "");
    let out = a.frame(54, 8);
    let row = out.split("\r\n").find(|r| r.contains("+short line")).expect("diff row");
    let shown = strip(row);
    assert_eq!(shown.trim_end_matches("\x1b[K").chars().count(), 54, "{row:?}");
    assert!(row.ends_with("\x1b[0m") || row.ends_with("\x1b[0m\x1b[K"), "the band stops short: {row:?}");
    assert!(row.starts_with("  \x1b[48;2;"), "the indent must stay clear: {row:?}");
    // Without colour the row keeps its sign and no padding.
    let mut b = app();
    b.push_diffs(vec!["+short line".into()], "");
    let plain = b.frame(54, 8);
    assert!(plain.contains("  +short line\x1b[K") || plain.contains("  +short line\r"), "{plain:?}");
}

#[test]
fn esc_on_a_prompt_stops_the_turn_but_n_only_declines() {
    let (tx, _cmds) = channel();
    let mut a = app();
    let (reply, answers) = channel();
    a.status.streaming = true;
    a.apply(Ev::Confirm { what: "run `make`".into(), reply, always: false });
    a.handle_key(Key::Char('n'), &tx);
    assert_eq!(answers.try_recv().ok(), Some(Answer::No));
    assert!(!a.cancel.load(Ordering::Relaxed), "n stopped the turn");
    let (reply, answers) = channel();
    a.apply(Ev::Confirm { what: "run `make`".into(), reply, always: false });
    a.handle_key(Key::Esc, &tx);
    assert_eq!(answers.try_recv().ok(), Some(Answer::No));
    assert!(a.cancel.load(Ordering::Relaxed), "Esc did not stop the turn");
    assert!(a.pending.is_none() && !a.quit);
    // A question stops the same way.
    let mut b = app();
    let (reply, answers) = channel();
    b.apply(Ev::Question { q: "which?".into(), opts: vec!["a".into(), "b".into(), "c".into()], descs: Vec::new(), multiple: false, reply });
    b.handle_key(Key::Esc, &tx);
    assert_eq!(answers.try_recv().ok(), Some(None));
    assert!(b.cancel.load(Ordering::Relaxed), "Esc on a question did not stop the turn");
}

#[test]
fn a_decline_shows_once_on_its_row() {
    let mut a = app();
    let (tx, _cmds) = channel();
    let (reply, answers) = channel();
    a.apply(Ev::ToolStart { name: "bash".into(), summary: "Delete README.md".into(), args: "{}".into() });
    a.apply(Ev::Confirm { what: "run `rm README.md`".into(), reply, always: false });
    a.handle_key(Key::Char('n'), &tx);
    assert_eq!(answers.try_recv().ok(), Some(Answer::No));
    let out = "refused: the user declined to run this command";
    a.apply(Ev::ToolEnd { ok: false, preview: out.into(), output: out.into(), detail: String::new(), ms: 1 });
    let screen = strip(&a.frame(60, 10));
    assert_eq!(screen.matches("declined").count(), 1, "{screen:?}");
    assert!(screen.contains("✗ Delete README.md") && screen.contains("  declined"), "{screen:?}");
    assert!(!screen.contains("refused:"), "{screen:?}");
    assert_eq!(a.entries.len(), 1);
}

#[test]
fn a_click_toggles_one_tool_row() {
    let mut a = app();
    a.cfg.mouse = true;
    let (tx, _cmds) = channel();
    a.begin_task();
    for cmd in ["false", "true"] {
        let args = format!("{{\"command\":\"{cmd}\"}}");
        a.apply(Ev::ToolStart { name: "bash".into(), summary: format!("run `{cmd}`"), args });
        a.apply(Ev::ToolEnd { ok: false, preview: String::new(), output: "exit 1".into(), detail: String::new(), ms: 1 });
    }
    a.frame(80, 14);
    click(&mut a, &tx, |t| matches!(t, Target::Tool(0)));
    assert!(a.entries[0].text.contains("$ false"), "the clicked row did not open");
    assert!(!a.entries[1].text.contains("$ true"), "the other row opened too");
    a.frame(80, 14);
    click(&mut a, &tx, |t| matches!(t, Target::Tool(1)));
    assert!(a.entries[1].text.contains("$ true"), "the newest row did not open");
    a.frame(80, 20);
    click(&mut a, &tx, |t| matches!(t, Target::Tool(0)));
    assert!(!a.entries[0].text.contains("$ false"), "a second click did not close it");
    // A picker over the rows takes the click; no row opens beneath it.
    a.handle_key(Key::Ctrl('e'), &tx);
    a.handle_key(Key::Ctrl('e'), &tx);
    assert!(!a.entries.iter().any(|e| e.text.contains("\n  $ ")));
    // Enough rows that the picker sits over them.
    for i in 0..12 {
        let args = format!("{{\"command\":\"step{i}\"}}");
        a.apply(Ev::ToolStart { name: "bash".into(), summary: format!("step {i}"), args });
        a.apply(Ev::ToolEnd { ok: true, preview: String::new(), output: "exit 0".into(), detail: String::new(), ms: 1 });
    }
    open_many(&mut a, 3);
    a.frame(80, 16);
    click(&mut a, &tx, |t| matches!(t, Target::Row(1)));
    assert!(!a.entries.iter().any(|e| e.text.contains("\n  $ ")), "a tool row opened under the picker");
    assert!(!matches!(a.pending, Some(Pending::Pick(_))), "the picker did not take the click");
    // With the mouse off there is nothing to click.
    a.cfg.mouse = false;
    a.frame(80, 20);
    assert!(!a.targets.iter().any(|t| matches!(t.3, Target::Tool(_))));
}

#[test]
fn an_exit_shows_the_tail_of_its_output() {
    let mut a = app();
    let end = |a: &mut App, output: &str| {
        a.entries.clear();
        a.apply(Ev::ToolStart { name: "bash".into(), summary: "run `make`".into(), args: "{}".into() });
        a.apply(Ev::ToolEnd { ok: false, preview: String::new(), output: output.into(), detail: String::new(), ms: 1 });
        strip(&a.frame(54, 14))
    };
    let short = end(&mut a, "exit 2\nmake: *** No rule\nto make target");
    assert!(short.contains("  exit 2\r\n  make: *** No rule\r\n  to make target"), "{short:?}");
    assert!(!short.contains("more above"), "{short:?}");
    let ten: String = (1..=10).map(|i| format!("\nline {i}")).collect();
    let long = end(&mut a, &format!("exit 1{ten}"));
    assert!(long.contains("  exit 1 · 7 more above (Ctrl+E)\r\n  line 8\r\n  line 9\r\n  line 10"), "{long:?}");
    assert!(!long.contains("line 7"), "{long:?}");
    assert!(long.lines().all(|l| l.trim_end_matches('\r').chars().count() <= 54));
}

#[test]
fn each_finish_has_its_mark_and_colour() {
    assert_eq!(Finish::of(true, "exit 0\nhi"), Finish::Done);
    assert_eq!(Finish::of(false, "exit 1\nno tests"), Finish::Exited);
    assert_eq!(Finish::of(false, "exit -1\ntimed out after 30s"), Finish::Failed);
    assert_eq!(Finish::of(false, "refused: outside the session"), Finish::Failed);
    assert_eq!(Finish::of(false, "refused: the user declined to run this command"), Finish::Declined);
    let mut a = colour_app();
    for (ok, output, mark, tone) in [
        (false, "exit 1", "!", Role::Info),
        (false, "refused: the user declined to run this command", "✗", Role::Context),
        (false, "refused: outside the session", "✗", Role::Error),
        (true, "exit 0", "✓", Role::Success),
    ] {
        a.entries.clear();
        a.apply(Ev::ToolStart { name: "bash".into(), summary: "run `x`".into(), args: "{}".into() });
        a.apply(Ev::ToolEnd { ok, preview: String::new(), output: output.into(), detail: String::new(), ms: 1 });
        let out = a.frame(60, 10);
        let want = format!("\x1b[1m{}", a.palette.paint(tone, mark));
        assert!(out.contains(&want), "{output}: {out:?}");
    }
}

fn picker() -> App {
    let mut a = app();
    a.pending = Some(Pending::Pick(Picker::new(
        "session",
        vec![PickItem {
            label: "abc123  3m  old title  4 msgs".into(),
            value: "abc123".into(),
            actions: Vec::new(),
            desc: String::new(),
        }],
        PickKind::Session,
    )));
    a
}

#[test]
fn picker_new_session_key() {
    let mut a = picker();
    let (tx, rx) = channel();
    a.handle_key(Key::Char('n'), &tx);
    assert!(matches!(rx.try_recv(), Ok(Cmd::NewSession)));
}

#[test]
fn picker_delete_needs_confirmation() {
    let mut a = picker();
    let (tx, rx) = channel();
    a.handle_key(Key::Char('d'), &tx);
    assert!(matches!(a.pending, Some(Pending::ConfirmDelete { .. })));
    let frame = strip(&a.frame(100, 30));
    assert!(frame.contains("delete abc123? y/N"), "{frame}");
    assert!(frame.contains("old title"), "the list vanished: {frame}");
    a.handle_key(Key::Char('y'), &tx);
    match rx.try_recv() {
        Ok(Cmd::Delete(id)) => assert_eq!(id, "abc123"),
        _ => panic!("expected delete"),
    }
    let picker = open_picker(&a);
    assert!(picker.items.is_empty(), "the deleted row stayed");
}

#[test]
fn the_delete_question_names_the_title() {
    let mut a = picker();
    let Some(Pending::Pick(list)) = a.pending.take() else { panic!("no picker") };
    a.pending = Some(Pending::ConfirmDelete {
        id: "51280befc".into(),
        title: "Rate my machine.".into(),
        picker: Box::new(list),
    });
    let frame = strip(&a.frame(100, 30));
    assert!(frame.contains("delete \"Rate my machine.\" (51280befc)? y/N"), "{frame}");
}

#[test]
fn text_prompts_use_the_block_caret() {
    let mut a = app();
    let (tx, _rx) = channel();
    a.run_command("rename", &tx);
    a.handle_key(Key::Char('x'), &tx);
    let frame = a.frame(80, 20);
    assert!(frame.contains("new title: x\x1b[7m \x1b[27m"), "{frame:?}");
    assert!(!strip(&frame).contains("x_"), "{frame:?}");
}

#[test]
fn picker_rename_edits_a_title() {
    let mut a = picker();
    let (tx, rx) = channel();
    a.handle_key(Key::Char('r'), &tx);
    for c in "new name".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    a.handle_key(Key::Enter, &tx);
    match rx.try_recv() {
        Ok(Cmd::Rename(id, title)) => {
            assert_eq!(id, "abc123");
            assert_eq!(title, "new name");
        }
        _ => panic!("expected rename"),
    }
}

#[test]
fn a_row_shortcut_runs_only_while_the_filter_is_empty() {
    let mut a = picker();
    let (tx, _rx) = channel();
    a.handle_key(Key::Char('r'), &tx);
    assert!(
        matches!(a.pending, Some(Pending::Prompt { .. })),
        "r did not rename the highlighted session"
    );

    let mut a = picker();
    a.handle_key(Key::Char('x'), &tx);
    a.handle_key(Key::Char('r'), &tx);
    let picker = open_picker(&a);
    assert_eq!(picker.filter, "xr", "r bypassed the filter");
    assert!(matches!(a.pending, Some(Pending::Pick(_))));
}

#[test]
fn the_footer_names_the_row_actions() {
    let mut a = picker();
    let frame = strip(&a.frame(80, 24));
    assert!(frame.contains("Enter choose"), "{frame}");
    assert!(frame.contains("Esc close"), "{frame}");
    assert!(frame.contains("n new"), "{frame}");
    assert!(frame.contains("r rename"), "{frame}");
    assert!(frame.contains("d delete"), "{frame}");
}

#[test]
fn slash_autocomplete_uses_the_picker() {
    let mut a = app();
    let (tx, _rx) = channel();
    a.handle_key(Key::Char('/'), &tx);
    assert_eq!(open_picker(&a).kind, PickKind::Command);

    a.handle_key(Key::Char('c'), &tx);
    a.handle_key(Key::Char('o'), &tx);
    {
        let picker = open_picker(&a);
        let labels: Vec<&str> = picker
            .matches()
            .iter()
            .map(|&i| picker.items[i].label.as_str())
            .collect();
        assert_eq!(labels.first().copied(), Some("/compact"), "{labels:?}");
        assert!(!labels.contains(&"/help"), "{labels:?}");
    }

    a.handle_key(Key::Tab, &tx);
    assert!(a.pending.is_none(), "Tab left the overlay open");
    assert_eq!(a.input, "/compact", "Tab did not complete the command");

    let (tx, rx) = channel();
    a.handle_key(Key::Enter, &tx);
    assert!(matches!(rx.try_recv(), Ok(Cmd::Compact)));
    assert!(a.input.is_empty(), "the input did not clear");
}

#[test]
fn tab_completes_and_opens_values() {
    let type_tab = |text: &str| {
        let mut a = app();
        let (tx, _rx) = channel();
        for c in text.chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        a.handle_key(Key::Tab, &tx);
        a
    };
    let a = type_tab("/the");
    assert_eq!(a.input, "/theme ");
    let picker = open_picker(&a);
    assert_eq!(picker.kind, PickKind::Theme);
    assert_eq!(picker.items[picker.index].value, a.palette.theme_name());

    let a = type_tab("/ren");
    assert_eq!(a.input, "/rename ");
    assert!(a.pending.is_none());

    let a = type_tab("/comp");
    assert_eq!(a.input, "/compact");
    assert!(a.pending.is_none());
    assert!(!a.entries.iter().any(|e| e.text.contains("compacting")));

    let a = type_tab("/sess");
    assert_eq!(a.input, "/sessions ");
    assert_eq!(open_picker(&a).kind, PickKind::Session);
}

#[test]
fn slash_autocomplete_runs_and_dismisses() {
    let mut a = app();
    let (tx, rx) = channel();
    a.handle_key(Key::Char('/'), &tx);
    a.handle_key(Key::Char('h'), &tx);
    a.handle_key(Key::Char('e'), &tx);
    a.handle_key(Key::Enter, &tx);
    assert!(a.pending.is_none(), "Enter left the overlay open");
    assert!(a.input.is_empty(), "the input did not clear");
    assert!(a.entries.iter().any(|e| e.text.contains("/help")));

    let mut a = app();
    let (tx, _rx) = channel();
    a.handle_key(Key::Char('/'), &tx);
    a.handle_key(Key::Char('c'), &tx);
    a.handle_key(Key::Char('o'), &tx);
    a.handle_key(Key::Esc, &tx);
    assert!(a.pending.is_none(), "Esc left the overlay open");
    assert_eq!(a.input, "/co", "Esc discarded the typed text");
}

#[test]
fn a_replay_draws_the_restored_conversation() {
    use crate::api::{Message, ToolCall};
    let mut asked = Message::assistant("");
    asked.tool_calls.push(ToolCall {
        id: "c1".into(),
        name: "bash".into(),
        arguments: "{\"command\":\"ls\"}".into(),
    });
    let history = vec![
        Message::system("prompt"),
        Message::user("list the files"),
        asked,
        Message::tool("c1", "a\nb"),
        Message::assistant("two files"),
    ];
    let mut a = app();
    a.push(Role::User, "old conversation");
    let (tx, rx) = channel();
    send_replay(&tx, &history, std::path::Path::new("/"));
    while let Ok(ev) = rx.try_recv() {
        a.apply(ev);
    }
    let frame = strip(&a.frame(80, 24));
    assert!(!frame.contains("old conversation"), "{frame}");
    let user = frame.find("> list the files").expect("user row");
    let tool = frame.find("▸ ").expect("tool row");
    let reply = frame.find("two files").expect("reply");
    assert!(user < tool && tool < reply, "{frame}");
    assert!(!frame.contains("prompt"), "{frame}");
    assert!(!frame.contains(" 0 tok"), "{frame}");
}

#[test]
fn a_typed_session_id_resumes() {
    let mut a = app();
    let (tx, rx) = channel();
    a.run_command("sessions 51280befc", &tx);
    match rx.try_recv() {
        Ok(Cmd::Resume(id)) => assert_eq!(id, "51280befc"),
        _ => panic!("expected a resume command"),
    }
}

#[test]
fn replies_use_one_form() {
    let mut a = app();
    let (tx, _rx) = channel();
    a.run_command("nope", &tx);
    let last = a.entries.last().unwrap();
    assert_eq!(last.role, Role::Error);
    assert!(last.text.contains("/nope"), "{}", last.text);
    a.run_command("think", &tx);
    let toast = a.status.toast.as_ref().map(|(t, _)| t.as_str());
    assert!(matches!(toast, Some("thinking: on" | "thinking: off")), "{toast:?}");
    a.run_command("verbosity loud", &tx);
    let last = a.entries.last().unwrap();
    assert_eq!(last.role, Role::Error);
    assert!(last.text.contains("low, medium, or high"), "{}", last.text);
}

#[test]
fn choosing_a_session_sends_resume() {
    let mut a = app();
    a.pending = Some(Pending::Pick(Picker::new(
        "session",
        vec![PickItem {
            label: "abc123  title  3 msgs".into(),
            value: "abc123".into(),
            actions: Vec::new(),
            desc: String::new(),
        }],
        PickKind::Session,
    )));
    let (tx, rx) = channel();
    a.handle_key(Key::Enter, &tx);
    match rx.try_recv() {
        Ok(Cmd::Resume(id)) => assert_eq!(id, "abc123"),
        _ => panic!("expected a resume command"),
    }
}

fn open_agent_picker(a: &mut App) {
    a.agents = ["default", "reviewer", "researcher", "writer"]
        .iter()
        .map(|n| (n.to_string(), format!("{n} profile")))
        .collect();
    let (tx, _rx) = channel();
    a.run_command("agent", &tx);
}

fn open_picker<'a>(a: &'a App) -> &'a Picker {
    match &a.pending {
        Some(Pending::Pick(picker)) => picker,
        _ => panic!("no picker is open"),
    }
}

#[test]
fn picker_filter_narrows_and_resets_the_highlight() {
    let mut a = app();
    open_agent_picker(&mut a);
    let (tx, _rx) = channel();
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Down, &tx);
    assert_eq!(open_picker(&a).index, 2, "Down did not move");

    for c in "rev".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    let picker = open_picker(&a);
    assert_eq!(picker.filter, "rev");
    let labels: Vec<&str> = picker
        .matches()
        .iter()
        .map(|&i| picker.items[i].label.as_str())
        .collect();
    assert_eq!(labels, vec!["reviewer"], "filter kept the wrong rows");
    assert_eq!(picker.index, 0, "filter did not reset the highlight");

    a.handle_key(Key::Backspace, &tx);
    assert_eq!(open_picker(&a).filter, "re");
    assert_eq!(open_picker(&a).matches().len(), 2);
}

#[test]
fn picker_pages_eight_rows_at_a_time() {
    let mut a = app();
    let items = (0..40)
        .map(|i| PickItem {
            label: format!("profile {i}"),
            value: format!("p{i}"),
            actions: Vec::new(),
            desc: String::new(),
        })
        .collect();
    a.pending = Some(Pending::Pick(Picker::new("agent", items, PickKind::Agent)));
    let (tx, _rx) = channel();

    a.handle_key(Key::PageDown, &tx);
    assert_eq!(open_picker(&a).index, 8, "PgDn did not move eight rows");
    assert_eq!(open_picker(&a).offset, 1, "the window did not scroll");

    a.handle_key(Key::PageDown, &tx);
    a.handle_key(Key::PageUp, &tx);
    assert_eq!(open_picker(&a).index, 8, "PgUp did not move eight rows");
}

#[test]
fn picker_enter_on_an_empty_result_does_nothing() {
    let mut a = app();
    open_agent_picker(&mut a);
    let (tx, rx) = channel();
    for c in "zzz".chars() {
        a.handle_key(Key::Char(c), &tx);
    }
    assert!(open_picker(&a).matches().is_empty());
    assert!(
        strip(&a.frame(80, 20)).contains("no matches"),
        "no no-match row"
    );
    a.handle_key(Key::Enter, &tx);
    assert!(a.pending.is_some(), "Enter closed an empty picker");
    assert!(rx.try_recv().is_err(), "Enter sent a command");
}

#[test]
fn picker_esc_cancels_without_changing_anything() {
    let mut a = app();
    open_agent_picker(&mut a);
    let (tx, rx) = channel();
    a.handle_key(Key::Char('w'), &tx);
    a.handle_key(Key::Esc, &tx);
    assert!(a.pending.is_none(), "the picker stayed open");
    assert!(rx.try_recv().is_err(), "Esc sent a command");
    assert_eq!(a.agent, "default", "Esc changed the agent");
}

#[test]
fn agent_picker_highlights_the_current_profile() {
    let mut a = app();
    a.agent = "writer".into();
    open_agent_picker(&mut a);
    let picker = open_picker(&a);
    assert_eq!(picker.selected().map(|i| i.label.as_str()), Some("writer"));
}

fn open_many(a: &mut App, count: usize) {
    let items = (0..count)
        .map(|i| PickItem {
            label: format!("profile {i}"),
            value: format!("p{i}"),
            actions: Vec::new(),
            desc: String::new(),
        })
        .collect();
    a.pending = Some(Pending::Pick(Picker::new("agent", items, PickKind::Agent)));
}

#[test]
fn overlay_is_forty_columns_in_a_narrow_terminal() {
    let mut a = app();
    open_many(&mut a, 3);
    let frame = a.frame(50, 24);
    let start = frame
        .find('╭')
        .unwrap_or_else(|| panic!("no overlay in {frame:?}"));
    let end = frame[start..]
        .find('╮')
        .map(|at| start + at)
        .unwrap_or_else(|| panic!("no top border in {frame:?}"));
    assert_eq!(
        frame[start..end].chars().count() + 1,
        40,
        "wrong box width: {:?}",
        &frame[start..end]
    );
}

#[test]
fn overlay_shows_eight_rows_and_scrolls() {
    let mut a = app();
    open_many(&mut a, 40);
    let frame = a.frame(100, 30);
    let rows = frame.matches("H│▸").count() + frame.matches("H│  ").count();
    assert_eq!(rows, 8, "wrong number of item rows in {frame:?}");
    assert!(strip(&frame).contains("profile 0"), "{frame:?}");

    let (tx, _rx) = channel();
    for _ in 0..8 {
        a.handle_key(Key::Down, &tx);
    }
    let scrolled = strip(&a.frame(100, 30));
    assert!(!scrolled.contains("profile 0"), "the window did not scroll");
    assert!(scrolled.contains("profile 8"), "{scrolled:?}");
}

#[test]
fn the_overlay_sets_title_position_and_keys_in_its_edges() {
    let mut a = app();
    open_many(&mut a, 40);
    let frame = strip(&a.frame(200, 30));
    let start = frame.find('╭').expect("top edge");
    let end = start + frame[start..].find('╮').expect("top corner");
    let top = &frame[start..end + '╮'.len_utf8()];
    assert!(top.starts_with("╭─ agent "), "{top}");
    assert!(top.ends_with(" 1/40 ─╮"), "{top}");
    assert_eq!(top.chars().count(), 72, "{top}");
    let bottom = &frame[frame.find('╰').expect("bottom edge")..];
    assert!(bottom.starts_with("╰─ Enter choose · Esc close "), "{bottom}");
    assert!(frame.contains("│ › "), "no filter row: {frame}");
}

#[test]
fn the_filter_cursor_follows_the_typed_text() {
    let mut a = app();
    open_many(&mut a, 3);
    let (tx, _rx) = channel();
    a.handle_key(Key::Char('r'), &tx);
    let (_, x, y, _) = a.overlay(100, 30).expect("overlay");
    // Box at column 15 (72 wide in 100): wall, " › ", then `r`.
    assert_eq!(x, 15 + 1 + 3 + 1);
    assert!(y >= 1);
}

#[test]
fn closing_the_picker_leaves_no_border() {
    let mut a = app();
    a.push(Role::User, "a transcript row");
    let before = a.frame(80, 20);
    open_many(&mut a, 40);
    let open = a.frame(80, 20);
    assert!(open.contains('╭'), "the picker did not draw");
    let (tx, _rx) = channel();
    a.handle_key(Key::Esc, &tx);
    let after = a.frame(80, 20);
    assert!(!after.contains('╭'), "border residue after close");
    assert_eq!(before, after, "closing the picker changed the frame");
}

fn long_app() -> App {
    let mut a = app();
    for i in 0..40 {
        a.push(Role::User, format!("line {i}"));
    }
    a
}

#[test]
fn scroll_detaches_from_the_tail() {
    let mut a = long_app();
    let (tx, _cmds) = channel();
    let tail = a.frame(60, 8);
    a.handle_key(Key::PageUp, &tx);
    let earlier = a.frame(60, 8);
    assert_ne!(tail, earlier, "PgUp drew the same rows");
    assert!(!a.follow, "PgUp did not detach");
    assert!(
        a.status_line(120).contains("↑"),
        "no unseen count: {}",
        a.status_line(120)
    );
}

#[test]
fn a_detached_view_holds_when_output_arrives() {
    let mut a = long_app();
    let (tx, _cmds) = channel();
    a.frame(60, 8);
    a.handle_key(Key::PageUp, &tx);
    let before = a.frame(60, 8).replace("\x1b[K", "");
    let was = a.unseen;
    a.push(Role::User, "brand new line");
    let after = a.frame(60, 8).replace("\x1b[K", "");
    assert!(!after.contains("brand new line"), "view jumped to the tail");
    // Turns are spaced now, so probe whichever row the page showed.
    let probe = (0..40)
        .map(|i| format!("> line {i}\r"))
        .find(|l| before.contains(l.as_str()))
        .unwrap_or_else(|| panic!("no row visible: {before:?}"));
    assert!(after.contains(&probe), "rows moved: {after:?}");
    assert!(a.unseen > was, "unseen count did not grow: {was} then {}", a.unseen);
}

#[test]
fn pgdn_at_the_newest_row_resumes_following() {
    let mut a = long_app();
    let (tx, _cmds) = channel();
    a.frame(60, 8);
    a.handle_key(Key::PageUp, &tx);
    a.frame(60, 8);
    a.handle_key(Key::PageDown, &tx);
    let out = a.frame(60, 8);
    assert!(a.follow, "PgDn did not resume following");
    assert!(out.contains("line 39"), "newest row missing: {out:?}");
    assert!(!a.status_line(120).contains("↑"));
}

#[test]
fn typing_resumes_following() {
    let mut a = long_app();
    let (tx, _cmds) = channel();
    a.frame(60, 8);
    a.handle_key(Key::PageUp, &tx);
    a.handle_key(Key::Char('x'), &tx);
    let out = a.frame(60, 8);
    assert!(a.follow, "editing did not resume following");
    assert!(out.contains("line 39"), "{out:?}");
}

#[test]
fn decodes_page_keys() {
    let mut buf = b"\x1b[5~\x1b[6~".to_vec();
    let keys = decode(&mut buf);
    assert_eq!(keys.len(), 2);
    assert!(matches!(keys[0], Key::PageUp));
    assert!(matches!(keys[1], Key::PageDown));
    assert!(buf.is_empty(), "page sequences leaked as text");
}

#[test]
fn decodes_control_keys() {
    let mut buf = vec![0x03, 0x0c, b'\t', 0x1b, b'[', b'H', 0x1b, b'[', b'F'];
    let keys = decode(&mut buf);
    assert!(matches!(keys[0], Key::Ctrl('c')));
    assert!(matches!(keys[1], Key::Ctrl('l')));
    assert!(matches!(keys[2], Key::Tab));
    assert!(matches!(keys[3], Key::Home));
    assert!(matches!(keys[4], Key::End));
}

#[test]
fn fit_truncates_to_width() {
    assert_eq!(fit("abcdef", 10), "abcdef");
    let short = fit("abcdefghij", 5);
    assert_eq!(short.chars().count(), 5);
    assert!(short.ends_with('…'));
}

#[test]
fn wrap_keeps_leading_spaces() {
    assert_eq!(wrap("  /new  start", 40), vec!["  /new  start"]);
}

#[test]
fn wrap_keeps_blank_lines() {
    assert_eq!(wrap("a\n\nb", 20).len(), 3);
}
#[test]
fn a_typed_command_on_the_deny_list_is_refused() {
    let mut a = app();
    a.run_local("echo mkfs-probe");
    let last = a.entries.last().unwrap();
    assert_eq!(last.role, Role::Error);
    assert!(last.text.contains("refused"), "{}", last.text);
    assert!(!last.text.contains("\nmkfs-probe"), "it ran: {}", last.text);
}

#[test]
fn a_shell_draft_shows_a_dollar_prompt() {
    let mut a = app();
    a.input = "!ls".into();
    a.cursor = 3;
    let out = a.frame(60, 12);
    assert!(out.contains("$ ls"), "no shell prompt: {out:?}");
    assert!(out.contains("shell · local"), "no shell status: {out:?}");
}

#[test]
fn a_normal_draft_keeps_the_arrow() {
    let mut a = app();
    a.input = "hello".into();
    a.cursor = 5;
    let out = a.frame(60, 12);
    assert!(out.contains("> hello"), "{out:?}");
    assert!(out.contains("deepseek-flash"), "{out:?}");
}

#[test]
fn ctrl_c_keeps_a_draft_and_never_quits_over_it() {
    let mut a = app();
    a.input = "half written".into();
    a.cursor = 4;
    let (tx, _cmds) = channel();
    a.handle_key(Key::Ctrl('c'), &tx);
    assert!(!a.quit, "a draft quit the session");
    assert_eq!(a.input, "half written", "Ctrl+C cleared the draft");
    assert_eq!(a.cursor, 4);
}

#[test]
fn ctrl_c_stops_the_turn_and_keeps_the_draft() {
    let mut a = app();
    a.status.streaming = true;
    a.input = "draft".into();
    a.cursor = 5;
    let (tx, _cmds) = channel();
    a.handle_key(Key::Ctrl('c'), &tx);
    assert!(!a.quit, "a draft quit the session");
    assert_eq!(a.input, "draft", "Ctrl+C cleared the draft");
    assert!(
        a.cancel.load(std::sync::atomic::Ordering::Relaxed),
        "Ctrl+C during a turn did not stop it"
    );
}

#[test]
fn ctrl_c_closes_a_picker_and_clears_its_draft() {
    let mut a = app();
    a.input = "/sess".into();
    a.cursor = 5;
    a.pending = Some(Pending::Pick(Picker::new(
        "commands",
        command_items(),
        PickKind::Command,
    )));
    let (tx, _cmds) = channel();
    a.handle_key(Key::Ctrl('c'), &tx);
    assert!(!a.quit, "a picker with a draft exited");
    assert!(a.pending.is_none(), "the picker stayed open");
    assert!(a.input.is_empty(), "the draft stayed: {:?}", a.input);
}

#[test]
fn ctrl_c_stops_a_running_turn_and_stays_open() {
    let mut a = app();
    a.status.streaming = true;
    let (tx, _cmds) = channel();
    a.handle_key(Key::Ctrl('c'), &tx);
    assert!(!a.quit, "Ctrl+C quit mid-turn");
    assert!(a.cancel.load(Ordering::Relaxed), "the turn was not stopped");
}

#[test]
fn the_loop_sleeps_when_nothing_moves() {
    let mut a = app();
    let quiet = Duration::from_secs(3);
    assert_eq!(a.loop_wait(quiet), Duration::from_secs(5));
    assert_eq!(a.loop_wait(Duration::from_millis(500)), Duration::from_millis(50), "a recent key");
    a.status.streaming = true;
    assert_eq!(a.loop_wait(quiet), Duration::from_millis(50));
    a.status.streaming = false;
    // An open prompt moves nothing, so the loop sleeps as when idle.
    let (reply, _answers) = channel();
    a.apply(Ev::Confirm { what: "run `ls`".into(), reply, always: false });
    assert_eq!(a.loop_wait(quiet), Duration::from_secs(5));
}

#[test]
fn an_idle_ctrl_c_asks_and_a_second_quits() {
    let mut a = app();
    let (tx, _cmds) = channel();
    a.handle_key(Key::Ctrl('c'), &tx);
    assert!(!a.quit, "one idle Ctrl+C quit");
    assert_eq!(a.entries.last().unwrap().text, "Ctrl+C again to quit");
    a.handle_key(Key::Ctrl('c'), &tx);
    assert!(a.quit, "the second Ctrl+C did not quit");
    // Too late for the second: it asks again.
    let mut b = app();
    b.quit_armed = Some(Instant::now() - Duration::from_secs(3));
    b.handle_key(Key::Ctrl('c'), &tx);
    assert!(!b.quit);
}

#[test]
fn a_confirm_asks_with_the_verb_alone() {
    let mut a = app();
    a.apply(Ev::ToolStart {
        name: "bash".into(),
        summary: "run `nmap -sV -T4 192.168.0.56`".into(),
        args: "{}".into(),
    });
    let (reply, _rx) = channel();
    a.apply(Ev::Confirm {
        what: "run `nmap -sV -T4 192.168.0.56`".into(),
        reply,
        always: false,
    });
    let out = strip(&a.frame(90, 12));
    assert!(out.contains("run it?   [y] yes  [n] no"), "no verb prompt: {out:?}");
    assert_eq!(
        out.matches("nmap -sV -T4 192.168.0.56").count(),
        1,
        "the command appears more than once: {out:?}"
    );
}

#[test]
fn an_unknown_verb_falls_back_to_approve() {
    let mut a = app();
    let (reply, _rx) = channel();
    a.apply(Ev::Confirm {
        what: "overwrite /etc/hosts".into(),
        reply,
        always: false,
    });
    assert!(strip(&a.frame(80, 12)).contains("approve?   [y] yes  [n] no"));
}

#[test]
fn ctrl_c_closes_a_picker_without_quitting() {
    let mut a = app();
    a.pending = Some(Pending::Pick(Picker::new(
        "session",
        vec![PickItem {
            label: "one".into(),
            value: "one".into(),
            actions: Vec::new(),
            desc: String::new(),
        }],
        PickKind::Session,
    )));
    let (tx, _cmds) = channel();
    a.handle_key(Key::Ctrl('c'), &tx);
    assert!(!a.quit, "Ctrl+C quit from a picker");
}

#[test]
fn ctrl_c_on_a_question_stops_the_turn() {
    let mut a = app();
    let (reply, _rx) = channel();
    a.pending = Some(Pending::Question {
        q: "which?".into(),
        opts: vec!["a".into(), "b".into()],
        reply,
        mode: AskMode::Choose,
        typed: String::new(),
    });
    let (tx, _cmds) = channel();
    a.handle_key(Key::Ctrl('c'), &tx);
    assert!(!a.quit, "Ctrl+C quit from a question");
    assert!(a.cancel.load(Ordering::Relaxed), "the turn was not stopped");
}

#[test]
fn ctrl_c_on_a_confirmation_stops_the_turn() {
    let mut a = app();
    let (reply, _rx) = channel();
    a.apply(Ev::Confirm {
        what: "run `ls`".into(),
        reply,
        always: false,
    });
    let (tx, _cmds) = channel();
    a.handle_key(Key::Ctrl('c'), &tx);
    assert!(!a.quit, "Ctrl+C quit from a confirmation");
    assert!(a.cancel.load(Ordering::Relaxed), "the turn was not stopped");
}

#[test]
fn ctrl_letters_survive_the_keyboard_protocol() {
    let mut buf = b"\x1b[99;5u\x1b[108;5u\x1b[100;5u".to_vec();
    let keys = decode(&mut buf);
    assert_eq!(keys.len(), 3, "{keys:?}");
    assert!(matches!(keys[0], Key::Ctrl('c')), "{:?}", keys[0]);
    assert!(matches!(keys[1], Key::Ctrl('l')), "{:?}", keys[1]);
    assert!(matches!(keys[2], Key::Ctrl('d')), "{:?}", keys[2]);
    assert!(buf.is_empty());
}

#[test]
fn ctrl_shift_letter_still_maps_to_ctrl() {
    let mut buf = b"\x1b[67;6u".to_vec();
    let keys = decode(&mut buf);
    assert!(matches!(keys[0], Key::Ctrl('c')), "{:?}", keys[0]);
}

#[cfg(target_os = "linux")]
#[test]
fn termios_matches_the_glibc_layout() {
    assert_eq!(std::mem::size_of::<Termios>(), 60);
    assert_eq!(std::mem::align_of::<Termios>(), 4);
}

#[test]
fn palette_names_are_never_cut() {
    let mut a = app();
    let (tx, _rx) = channel();
    a.handle_key(Key::Ctrl('p'), &tx);
    let Some(Pending::Pick(p)) = &a.pending else { panic!("no palette") };
    for (i, item) in p.items.iter().enumerate() {
        let row = picker_row(p, p.index, i, 56, false).join("");
        assert!(row.contains(&item.label), "{} is cut: {row:?}", item.label);
    }
}

#[test]
fn reload_rereads_the_config() {
    let dir = scratch("reload-config");
    let path = dir.join("config");
    std::fs::write(&path, "max_steps = 5\n").unwrap();
    let cfg = Config::load(&path).unwrap();
    let mut a = app();
    a.cfg = cfg.clone();
    let session = Session::new(
        cfg,
        dir.clone(),
        dir.join("out"),
        String::new(),
        crate::store::Agent::builtin(),
        Vec::new(),
        std::sync::Arc::new(std::sync::atomic::AtomicU16::new(80)),
    );
    let (tx, rx, _) = spawn_worker(session, dir.join("sessions"));
    std::fs::write(&path, "max_steps = 5\nallow_sudo = true\ntypo = 1\n").unwrap();
    tx.send(Cmd::Reload).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut said = Vec::new();
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ev::Done(_)) => break,
            Ok(Ev::Info(t)) => said.push(t),
            Ok(ev) => a.apply(ev),
            Err(_) => {}
        }
    }
    assert!(said.iter().any(|t| t == "allow_sudo: unset → true"), "{said:?}");
    assert!(said.iter().any(|t| t == "config: unknown key typo ignored"), "{said:?}");
    assert!(a.cfg.allow_sudo, "the chat kept the old config");
    let _ = tx.send(Cmd::Quit);
}

#[test]
fn a_paste_decodes_whole_even_across_reads() {
    let mut buf = b"x\x1b[200~one\rtw".to_vec();
    assert_eq!(decode(&mut buf), vec![Key::Char('x')], "a part paste leaked keys");
    buf.extend_from_slice(b"o\x1b[201~y");
    assert_eq!(decode(&mut buf), vec![Key::Paste("one\ntwo".into()), Key::Char('y')]);
    assert!(enter_sequence(true).contains("\x1b[?2004h") && enter_sequence(false).contains("\x1b[?2004h"));
    assert!(ALT_OFF.contains("\x1b[?2004l") && RESTORE.contains("\x1b[?2004l"));
}

#[test]
fn a_paste_fills_the_draft_and_never_answers() {
    let mut a = app();
    let (tx, rx) = channel();
    a.handle_key(Key::Paste("one\ntwo".into()), &tx);
    assert_eq!((a.input.as_str(), a.cursor), ("one\ntwo", 7));
    assert!(rx.try_recv().is_err() && a.entries.is_empty(), "the paste was sent");
    let (reply, answers) = channel();
    a.apply(Ev::Confirm { what: "run `make`".into(), reply, always: false });
    a.handle_key(Key::Paste("y".into()), &tx);
    assert!(matches!(a.pending, Some(Pending::Confirm { .. })), "a paste answered");
    assert!(answers.try_recv().is_err());
    // Nor does it act on a picker's rows.
    a.pending = None;
    a.run_command("mode", &tx);
    a.handle_key(Key::Paste("da".into()), &tx);
    assert!(matches!(&a.pending, Some(Pending::Pick(p)) if p.kind == PickKind::Mode), "a paste acted on the picker");
    // A question takes it as a typed answer, digits included.
    a.pending = None;
    let got = asked(&mut a);
    a.handle_key(Key::Paste("3 files\nonly".into()), &tx);
    a.handle_key(Key::Enter, &tx);
    assert_eq!(got.recv().unwrap().as_deref(), Some("3 files only"));
}

#[test]
fn drags_decode_and_copy_on_release() {
    let mut buf = b"\x1b[<32;5;3M\x1b[<0;9;3m".to_vec();
    assert_eq!(decode(&mut buf), vec![Key::Drag(5, 3), Key::Release(9, 3)]);
    assert!(MOUSE_ON.contains("\x1b[?1002h"));
    let mut a = app();
    let (tx, _rx) = channel();
    a.push(Role::Assistant, "hello world, and more");
    let shown = a.frame(40, 10);
    let rows: Vec<String> = shown.split("\r\n").map(strip).collect();
    let y = rows.iter().position(|r| r.contains("hello world")).expect("the row") + 1;
    let x = rows[y - 1].find("hello").unwrap() + 1;
    let (x, y) = (x as u16, y as u16);
    a.handle_key(Key::Click(x, y), &tx);
    a.handle_key(Key::Drag(x + 10, y), &tx);
    assert!(a.frame(40, 10).contains("\x1b[7mhello world\x1b[27m"), "no highlight");
    a.handle_key(Key::Release(x + 10, y), &tx);
    assert_eq!(a.clip, "hello world");
    assert_eq!(a.status.toast.as_ref().map(|(t, _)| t.as_str()), Some("copied 11 characters"));
    assert!(!a.frame(40, 10).contains("\x1b[7mhello"), "the highlight stayed");
    // A click alone copies nothing.
    a.clip.clear();
    a.handle_key(Key::Click(x, y), &tx);
    a.handle_key(Key::Release(x, y), &tx);
    assert!(a.clip.is_empty());
}

#[test]
fn a_retry_counts_down_and_a_failure_offers_ctrl_r() {
    let mut a = app();
    a.status.streaming = true;
    a.apply(Ev::Retry { secs: 4, attempt: 2, of: 5 });
    assert_eq!(a.status_line(80).trim_end(), " retrying in 4s · 2/5 · Esc stop");
    a.apply(Ev::Retry { secs: 0, attempt: 2, of: 5 });
    assert!(!a.status_line(80).contains("retrying"), "the countdown stayed");
    a.apply(Ev::Retry { secs: 2, attempt: 3, of: 5 });
    a.apply(Ev::Done(Some("HTTP 401: bad key".into())));
    assert_eq!(a.entries.last().unwrap().text, "HTTP 401: bad key · Ctrl+R retry");
    assert!(!a.status_line(80).contains("retrying"));
}

#[test]
fn a_toast_shows_then_goes() {
    let mut a = app();
    let (tx, _rx) = channel();
    let before = a.status_line(80);
    let rows = a.entries.len();
    a.run_command("verbosity high", &tx);
    assert_eq!(a.entries.len(), rows, "the confirmation went to the transcript");
    assert_eq!(a.status_line(80).trim_end(), " verbosity: high");
    assert!(a.loop_wait(Duration::from_secs(9)) <= TOAST, "the loop sleeps past the toast");
    a.status.streaming = true;
    assert!(a.loop_wait(Duration::from_secs(9)) <= Duration::from_millis(50), "a toast slowed a streaming turn");
    a.status.streaming = false;
    a.status.toast.as_mut().unwrap().1 -= TOAST;
    assert!(a.expire_toast(), "a due toast stayed");
    assert_eq!(a.status_line(80), before);
    assert!(!a.expire_toast());
}

#[test]
fn the_title_bar_names_the_session_and_the_time() {
    let mut a = app();
    let bar = strip(&a.title_bar(40));
    assert_eq!(bar.chars().count(), 40);
    assert!(bar.starts_with(" imp "), "{bar:?}");
    let time = clock();
    assert!(time.len() == 5 && time.as_bytes()[2] == b':', "{time:?}");
    assert!(bar.ends_with(&format!(" {time} ")), "{bar:?}");
    a.push(Role::User, "a very long session title that cannot fit in forty columns");
    let bar = strip(&a.title_bar(40));
    assert!(bar.contains('…') && bar.ends_with(&format!(" {time} ")), "{bar:?}");
    assert!(!strip(&a.frame(40, 7)).contains(&time), "a title bar on a 7-row screen");
    assert!(!a.clock_due(), "a repaint with the minute unchanged");
    a.clock_shown = "99:99".into();
    assert!(a.clock_due());
}

#[test]
fn effort_is_set_shown_and_noted_once() {
    let mut a = app();
    let (tx, rx) = channel();
    a.cfg.thinking = false;
    a.run_command("effort high", &tx);
    assert_eq!(a.cfg.effort.as_deref(), Some("high"));
    assert_eq!(a.status.toast.as_ref().map(|(t, _)| t.as_str()), Some("effort: high · applies with thinking on"));
    assert!(matches!(rx.try_recv(), Ok(Cmd::SetEffort(Some(l))) if l == "high"));
    a.run_command("effort", &tx);
    assert_eq!(a.status.toast.as_ref().map(|(t, _)| t.as_str()), Some("effort: high"));
    a.run_command("effort default", &tx);
    assert_eq!(a.cfg.effort, None);
    a.run_command("effort huge", &tx);
    assert_eq!(a.entries.last().unwrap().role, Role::Error);
    a.entries.clear();
    a.cfg.set_provider(crate::providers::Provider::Ollama);
    a.run_command("effort low", &tx);
    a.run_command("effort medium", &tx);
    let notes = a.entries.iter().filter(|e| e.text.contains("has no effort setting")).count();
    assert_eq!(notes, 1, "the note was not shown exactly once");
}

#[test]
fn a_new_session_takes_the_reloaded_config() {
    let mut a = multi_app("reload-new");
    let new = Config::from_str("allow_sudo = true\n").unwrap();
    a.apply(Ev::Config(Box::new(new)));
    assert!(a.cfg.allow_sudo);
    let s = a.fresh_session().expect("a factory");
    assert!(s.cfg.allow_sudo, "a new session kept the startup config");
}

#[test]
fn the_clock_matches_date_in_this_zone() {
    // Run under any `TZ`: both sides read the same zone.
    let date = std::process::Command::new("date").arg("+%H:%M").output().unwrap();
    let date = String::from_utf8_lossy(&date.stdout).trim().to_string();
    let ours = clock();
    // A minute may turn between the two reads.
    let again = std::process::Command::new("date").arg("+%H:%M").output().unwrap();
    let again = String::from_utf8_lossy(&again.stdout).trim().to_string();
    assert!(ours == date || ours == again, "clock {ours} but date {date}");
}

#[test]
fn the_title_gradient_is_optional_and_needs_truecolor() {
    let mut a = colour_app();
    a.palette.truecolor = true;
    let backgrounds = |bar: &str| {
        let mut seen: Vec<&str> = bar.split("\x1b[48;2;").skip(1).map(|p| p.split('m').next().unwrap_or("")).collect();
        seen.dedup();
        seen.len()
    };
    assert_eq!(backgrounds(&a.title_bar(40)), 1, "flat by default");
    a.cfg.title_gradient = true;
    let bar = a.title_bar(40);
    assert!(backgrounds(&bar) > 10, "no gradient: {bar:?}");
    assert_eq!(strip(&bar).chars().count(), 40, "the gradient changed the text");
    a.palette.truecolor = false;
    assert!(!a.title_bar(40).contains("\x1b[48;2;"), "a gradient without truecolor");
}

#[test]
fn the_status_line_shows_the_context_percent_when_the_window_is_known() {
    let mut a = app();
    a.cfg.context_window = Some(200_000);
    a.status.tokens = 12_300;
    assert!(a.status_line(120).contains("12.3K tok (6%)"), "{}", a.status_line(120));
    a.status.tokens = 1_200_000;
    assert!(a.status_line(120).contains("1.2M tok (100%)"), "{}", a.status_line(120));
}

#[test]
fn the_status_line_omits_the_percent_when_the_window_is_unknown() {
    let mut a = app();
    a.cfg.model = "some-other-model".into();
    a.status.tokens = 7_700;
    let line = a.status_line(120);
    assert!(line.contains("7.7K tok") && !line.contains('%'), "{line}");
}

#[test]
fn the_status_line_uses_the_configured_window() {
    let mut a = app();
    a.cfg.model = "some-other-model".into();
    a.cfg.context_window = Some(32_000);
    a.status.tokens = 8_000;
    assert!(a.status_line(120).contains("8K tok (25%)"), "{}", a.status_line(120));
}

#[test]
fn the_status_line_keeps_the_percent_inside_a_narrow_width() {
    let mut a = app();
    a.status.tokens = 12_300;
    assert!(a.status_line(40).chars().count() <= 40, "{}", a.status_line(40));
}

fn focus_setting(a: &mut App, key: &str) -> usize {
    let at = match &a.pending {
        Some(Pending::Pick(p)) => p.items.iter().position(|i| i.value == key).expect("no such setting row"),
        _ => panic!("no picker is open"),
    };
    if let Some(Pending::Pick(p)) = &mut a.pending {
        p.index = at;
    }
    at
}

fn setting_desc(a: &App, key: &str) -> String {
    let picker = open_picker(a);
    picker.items.iter().find(|i| i.value == key).expect("no such setting row").desc.clone()
}

#[test]
fn settings_lists_ten_settings_with_their_values() {
    let mut a = app();
    let (tx, _rx) = channel();
    a.run_command("settings", &tx);
    let picker = open_picker(&a);
    assert_eq!(picker.kind, PickKind::Settings);
    // Ten from the first form, four enumerable rows, the API key, and nine text or number rows.
    assert_eq!(picker.items.len(), 24);
    assert_eq!(setting_desc(&a, "thinking"), "off");
    assert_eq!(setting_desc(&a, "effort"), "default");
}

#[test]
fn enter_toggles_thinking_and_the_row_shows_it() {
    let mut a = app();
    let (tx, rx) = channel();
    a.run_command("settings", &tx);
    focus_setting(&mut a, "thinking");
    a.handle_key(Key::Enter, &tx);
    assert!(a.cfg.thinking, "thinking did not switch on");
    assert_eq!(setting_desc(&a, "thinking"), "on");
    assert!(
        matches!(rx.try_recv(), Ok(Cmd::SetConfig(k, v)) if k == "thinking" && v == "on"),
        "the session was not told"
    );
}

#[test]
fn space_and_the_arrows_step_a_value_and_space_does_not_type() {
    let mut a = app();
    let (tx, _rx) = channel();
    a.run_command("settings", &tx);
    focus_setting(&mut a, "effort");
    a.handle_key(Key::Right, &tx);
    a.handle_key(Key::Right, &tx);
    assert_eq!(setting_desc(&a, "effort"), "medium");
    a.handle_key(Key::Left, &tx);
    assert_eq!(setting_desc(&a, "effort"), "low");
    a.handle_key(Key::Char(' '), &tx);
    assert_eq!(setting_desc(&a, "effort"), "medium");
    assert_eq!(open_picker(&a).filter, "", "Space went into the filter");
}

#[test]
fn a_change_is_saved_to_the_config_file_keeping_comments() {
    let dir = std::env::temp_dir().join(format!("imp-settings-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config");
    std::fs::write(&path, "# keep\nthinking=false\n").unwrap();
    let mut a = app();
    a.cfg.file = Some(path.clone());
    let (tx, _rx) = channel();
    a.run_command("settings", &tx);
    focus_setting(&mut a, "thinking");
    a.handle_key(Key::Enter, &tx);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "# keep\nthinking = on\n");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn esc_closes_the_form_and_keeps_the_values() {
    let mut a = app();
    let (tx, _rx) = channel();
    a.run_command("settings", &tx);
    focus_setting(&mut a, "thinking");
    a.handle_key(Key::Enter, &tx);
    a.handle_key(Key::Esc, &tx);
    assert!(a.pending.is_none(), "the form stayed open");
    assert!(a.cfg.thinking, "Esc reverted the value");
}

#[test]
fn the_status_line_names_auto_and_not_normal() {
    let mut a = app();
    a.cfg.set_permissions(crate::config::Permissions::Auto);
    assert!(a.status_line(100).starts_with("deepseek-flash · auto · 0 tok"), "{}", a.status_line(100));
    a.cfg.set_permissions(crate::config::Permissions::Normal);
    assert!(!a.status_line(100).contains("auto"), "{}", a.status_line(100));
}

fn open_question_with_queue(a: &mut App, queued: &[&str]) -> Receiver<Option<String>> {
    let (reply, rx) = channel();
    a.status.streaming = true;
    a.queued = queued.iter().map(|s| s.to_string()).collect();
    a.pending = Some(Pending::Question {
        q: "which package manager?".into(),
        opts: vec!["npm".into(), "pnpm".into()],
        reply,
        mode: AskMode::Choose,
        typed: String::new(),
    });
    rx
}

#[test]
fn a_question_hint_counts_the_queue_and_the_rule_offers_no_edit() {
    let (mut a, _cmds, _ev) = worker_app();
    let _rx = open_question_with_queue(&mut a, &["use pnpm", "also add tests"]);
    let shown = strip(&a.frame(70, 14));
    assert!(shown.contains("type to answer   Esc stop · 2 queued"), "{shown:?}");
    assert!(!shown.contains("↑ edit queue"), "the rule offers an edit that ↑ does not do: {shown:?}");
}

#[test]
fn a_question_never_takes_a_queued_message_as_its_answer() {
    let (mut a, cmds, _ev) = worker_app();
    let tx = a.worker.as_ref().unwrap().0.clone();
    let rx = open_question_with_queue(&mut a, &["use pnpm"]);
    a.handle_key(Key::Enter, &tx);
    assert!(rx.try_recv().is_err(), "Enter answered the question");
    assert_eq!(a.queued, vec!["use pnpm"], "the queue changed");
    assert!(matches!(a.pending, Some(Pending::Question { .. })), "the question closed");
    assert!(cmds.try_recv().is_err(), "a message was sent");
}

#[test]
fn esc_on_a_question_keeps_the_queue_and_starts_no_turn() {
    let (mut a, cmds, _ev) = worker_app();
    let tx = a.worker.as_ref().unwrap().0.clone();
    let rx = open_question_with_queue(&mut a, &["use pnpm", "also add tests"]);
    a.handle_key(Key::Esc, &tx);
    assert!(matches!(rx.try_recv(), Ok(None)), "Esc did not decline the question");
    assert_eq!(a.queued, vec!["use pnpm", "also add tests"], "Esc changed the queue");
    a.apply(Ev::Done(None));
    assert!(cmds.try_recv().is_err(), "a stopped turn started the queue");
    assert_eq!(a.queued, vec!["use pnpm", "also add tests"]);
}

#[test]
fn after_the_answer_the_oldest_queued_message_starts_when_the_turn_ends() {
    let (mut a, cmds, _ev) = worker_app();
    let tx = a.worker.as_ref().unwrap().0.clone();
    let rx = open_question_with_queue(&mut a, &["use pnpm", "also add tests"]);
    a.handle_key(Key::Char('2'), &tx);
    assert!(matches!(rx.try_recv(), Ok(Some(t)) if t == "pnpm"), "the answer was not sent");
    a.apply(Ev::Done(None));
    assert!(matches!(cmds.try_recv(), Ok(Cmd::Prompt(t)) if t == "use pnpm"), "the oldest message did not start");
    assert_eq!(a.queued, vec!["also add tests"]);
}

#[test]
fn the_title_bar_shows_the_cut_first_message() {
    let mut a = app();
    a.push(Role::User, "Can you please look at why the rsync test keeps failing on the Pi when the network is slow");
    let shown = strip(&a.frame(60, 10));
    assert!(shown.contains(" look at why the rsync test keeps… "), "{shown:?}");
}

#[test]
fn a_rename_shows_in_the_title_bar() {
    let mut a = app();
    a.push(Role::User, "Can you please look at why the rsync test keeps failing");
    let (tx, _cmds) = channel();
    a.run_command("rename fix sync", &tx);
    let shown = strip(&a.frame(60, 10));
    let title_row = shown.split("\r\n").next().unwrap_or("");
    assert!(title_row.contains(" fix sync "), "the rename did not show: {title_row:?}");
    assert!(!title_row.contains("look at why"), "the old title stayed: {title_row:?}");
}

#[test]
fn a_summary_replaces_the_cut_title_unless_it_was_renamed() {
    let mut a = app();
    let long = "Can you please look at why the rsync test keeps failing on the Pi when the network is slow";
    a.push(Role::User, long);
    a.apply(Ev::Title("Flaky rsync test on slow network".into()));
    let shown = strip(&a.frame(70, 10));
    assert!(shown.split("\r\n").next().unwrap_or("").contains("Flaky rsync test on slow network"), "{shown:?}");

    let mut b = app();
    b.push(Role::User, long);
    let (tx, _cmds) = channel();
    b.run_command("rename fix sync", &tx);
    b.apply(Ev::Title("Flaky rsync test on slow network".into()));
    let shown = strip(&b.frame(70, 10));
    assert!(shown.split("\r\n").next().unwrap_or("").contains(" fix sync "), "a summary replaced a rename: {shown:?}");
}

fn provider_form(a: &App) -> (Vec<(String, String)>, usize) {
    match &a.pending {
        Some(Pending::ProviderForm { fields, at, .. }) => (fields.clone(), *at),
        _ => panic!("no provider form is open"),
    }
}

fn type_text(a: &mut App, tx: &Sender<Cmd>, text: &str) {
    for c in text.chars() {
        a.handle_key(Key::Char(c), tx);
    }
}

#[test]
fn up_and_shift_tab_move_back_and_keep_every_fields_text() {
    let (mut a, _config) = provider_app();
    let (tx, _rx) = channel();
    a.run_command("provider add work", &tx);
    type_text(&mut a, &tx, "http://box/v1");
    a.handle_key(Key::Enter, &tx);
    type_text(&mut a, &tx, "m1");
    a.handle_key(Key::Enter, &tx);
    a.handle_key(Key::Right, &tx);
    a.handle_key(Key::Right, &tx);
    a.handle_key(Key::Enter, &tx);
    assert_eq!(provider_form(&a).1, 3, "Enter did not reach key_env");

    a.handle_key(Key::Up, &tx);
    a.handle_key(Key::Up, &tx);
    let (fields, at) = provider_form(&a);
    assert_eq!(at, 1, "↑ did not move back to model");
    assert_eq!(fields[1].1, "m1");

    a.handle_key(Key::Backspace, &tx);
    a.handle_key(Key::Char('2'), &tx);
    let (fields, _) = provider_form(&a);
    assert_eq!(fields[1].1, "m2", "the edited model was not kept");
    assert_eq!(fields[0].1, "http://box/v1", "base changed");
    assert_eq!(fields[2].1, "openai", "dialect changed");

    a.handle_key(Key::BackTab, &tx);
    assert_eq!(provider_form(&a).1, 0, "Shift+Tab did not move back");
    a.handle_key(Key::Down, &tx);
    assert_eq!(provider_form(&a).1, 1, "↓ did not move forward");
}

#[test]
fn the_provider_form_stops_at_the_first_and_last_field() {
    let (mut a, _config) = provider_app();
    let (tx, _rx) = channel();
    a.run_command("provider add work", &tx);
    a.handle_key(Key::Up, &tx);
    assert_eq!(provider_form(&a).1, 0, "↑ went past the first field");
    for _ in 0..6 {
        a.handle_key(Key::Down, &tx);
    }
    assert_eq!(provider_form(&a).1, 5, "↓ went past the last field");
}

#[test]
fn esc_on_the_provider_form_cancels_everything() {
    let (mut a, config) = provider_app();
    let (tx, _rx) = channel();
    a.run_command("provider add work", &tx);
    type_text(&mut a, &tx, "http://box/v1");
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Esc, &tx);
    assert!(a.pending.is_none(), "Esc left the form open");
    assert!(!std::fs::read_to_string(&config).unwrap_or_default().contains("work"), "Esc saved the provider");
}

#[test]
fn provider_names_stay_whole_and_the_url_is_what_gets_cut() {
    let items = vec![
        ("deepseek", "current · https://api.deepseek.com", "deepseek-flash"),
        ("anthropic", "https://api.anthropic.com", "claude-opus-5-5"),
        ("gemini", "https://generativelanguage.googleapis.com", "gemini-2.5-pro"),
        ("ollama", "http://localhost:11434/v1", "llama3.2"),
        ("custom", "http://localhost:8000/v1", "local-model"),
    ]
    .into_iter()
    .map(|(name, base, model)| PickItem {
        label: name.into(),
        value: name.into(),
        actions: Vec::new(),
        desc: format!("{base}  {model}"),
    })
    .collect();
    let picker = Picker::new("provider", items, PickKind::Provider);
    let at = picker.items.iter().position(|i| i.label == "anthropic").unwrap();
    let row = picker_row(&picker, at, at, 68, false).remove(0);
    // Check the label column itself: the URL also contains "anthropic".
    assert!(row.starts_with("  anthropic "), "the provider name was clipped: {row:?}");
    assert!(row.chars().count() <= 68, "{row:?}");
}


#[test]
fn a_waiting_session_outranks_running_failed_and_unseen() {
    assert_eq!(state_mark(true, true, true, true, '⠋'), Some('?'));
    assert_eq!(state_mark(false, true, true, true, '⠋'), Some('⠋'));
    assert_eq!(state_mark(false, false, true, true, '⠋'), Some('!'));
    assert_eq!(state_mark(false, false, false, true, '⠋'), Some('•'));
    assert_eq!(state_mark(false, false, false, false, '⠋'), None, "an idle session has a mark");
}

#[test]
fn the_sessions_modal_lists_a_blank_session() {
    let mut a = multi_app("sessions-blank");
    let dir = a.sessions_dir.clone();
    std::fs::create_dir_all(&dir).unwrap();
    let _blank = crate::store::Log::create(&dir, &dir, "deepseek-flash");
    let (tx, _rx) = channel();
    a.run_command("sessions", &tx);
    let items = open_picker(&a).items.clone();
    assert_eq!(items.len(), 1, "the blank session is missing from the modal");
    assert!(items[0].label.contains("(untitled)"), "{:?}", items[0].label);
}

#[test]
fn marks_are_written_once_and_repainting_does_not_stack_them() {
    let row = |id: &str| PickItem { label: id.into(), value: id.into(), actions: Vec::new(), desc: "3 msgs".into() };
    let mut items = vec![row("abc"), row("def")];
    let marks = vec![("abc".to_string(), '?')];
    mark_rows(&mut items, &marks);
    assert_eq!(items[0].desc, "? 3 msgs");
    assert_eq!(items[1].desc, "  3 msgs", "an idle row lost its column");
    mark_rows(&mut items, &marks);
    mark_rows(&mut items, &marks);
    assert_eq!(items[0].desc, "? 3 msgs", "repaints stacked marks");
    assert_eq!(items[1].desc, "  3 msgs");
    mark_rows(&mut items, &[("def".to_string(), '•')]);
    assert_eq!(items[0].desc, "  3 msgs", "a cleared mark stayed");
    assert_eq!(items[1].desc, "• 3 msgs");
}

fn toast_text(a: &App) -> Option<String> {
    a.status.toast.as_ref().map(|(text, _)| text.clone())
}

#[test]
fn a_background_reply_raises_a_toast_with_position_and_title() {
    let (mut a, _answers) = inbox_app("toast-reply");
    a.status.toast = None;
    a.apply_to(0, Ev::Done(None));
    assert_eq!(toast_text(&a).as_deref(), Some("1/3 run the tests: replied"));
}

#[test]
fn the_session_in_view_raises_no_toast() {
    let (mut a, _answers) = inbox_app("toast-view");
    a.status.toast = None;
    let current = a.at;
    a.apply_to(current, Ev::Done(None));
    assert!(toast_text(&a).is_none(), "the session in view raised a toast: {:?}", toast_text(&a));
}

#[test]
fn the_newest_background_toast_replaces_the_older() {
    let (mut a, _answers) = inbox_app("toast-newest");
    a.apply_to(0, Ev::Done(None));
    a.apply_to(1, Ev::Woke("check-in: job 3".into()));
    assert_eq!(toast_text(&a).as_deref(), Some("2/3 summarise the log: ⏰ check-in: job 3"));
}

#[test]
fn a_background_job_notice_raises_a_toast_and_the_chat_keeps_its_row() {
    let (mut a, _answers) = inbox_app("toast-job");
    a.status.toast = None;
    a.apply_to(0, Ev::Notice("Background job 4 finished with exit 0".into()));
    assert_eq!(
        toast_text(&a).as_deref(),
        Some("1/3 run the tests: Background job 4 finished with exit 0")
    );
    let current = a.at;
    a.apply_to(current, Ev::Notice("Background job 5 finished with exit 0".into()));
    assert!(
        a.entries.iter().any(|e| e.text == "Background job 5 finished with exit 0"),
        "the chat lost the job row"
    );
}

fn provider_row(a: &mut App, name: &str) {
    match &mut a.pending {
        Some(Pending::Pick(p)) => {
            p.index = p.items.iter().position(|i| i.value == name).expect("no provider row");
        }
        _ => panic!("no picker is open"),
    }
}

#[test]
fn e_on_a_built_in_provider_opens_its_defaults_as_a_named_copy_form() {
    let (mut a, _config) = provider_app();
    let (tx, _rx) = channel();
    a.run_command("provider", &tx);
    provider_row(&mut a, "deepseek");
    a.handle_key(Key::Char('e'), &tx);
    match &a.pending {
        Some(Pending::ProviderForm { name, fields, at }) => {
            assert_eq!(name, "deepseek-custom");
            assert_eq!(fields[0].1, crate::providers::Provider::DeepSeek.profile().base_url);
            assert_eq!(fields[1].1, crate::providers::Provider::DeepSeek.profile().model);
            assert_eq!(fields[2].1, "deepseek");
            assert_eq!(*at, 0);
        }
        _ => panic!("e did not open the provider form"),
    }
}

#[test]
fn e_on_a_custom_provider_opens_its_saved_profile() {
    let (mut a, _config) = provider_app();
    a.cfg.named.push(crate::config::NamedProvider {
        name: "work".into(),
        base: "http://box/v1".into(),
        model: "m1".into(),
        key_env: String::new(),
        dialect: "openai".into(),
        models: Vec::new(),
    });
    let (tx, _rx) = channel();
    a.run_command("provider", &tx);
    provider_row(&mut a, "work");
    a.handle_key(Key::Char('e'), &tx);
    match &a.pending {
        Some(Pending::ProviderForm { name, fields, .. }) => {
            assert_eq!(name, "work");
            assert_eq!(fields[0].1, "http://box/v1");
            assert_eq!(fields[1].1, "m1");
            assert_eq!(fields[2].1, "openai");
        }
        _ => panic!("e did not open the saved profile"),
    }
}

#[test]
fn saving_a_built_in_change_adds_a_named_copy_and_keeps_the_built_in() {
    let (mut a, config) = provider_app();
    let (tx, _rx) = channel();
    a.run_command("provider", &tx);
    provider_row(&mut a, "deepseek");
    a.handle_key(Key::Char('e'), &tx);
    a.handle_key(Key::Down, &tx);
    for _ in 0..crate::providers::Provider::DeepSeek.profile().model.len() {
        a.handle_key(Key::Backspace, &tx);
    }
    type_text(&mut a, &tx, "deepseek-pro");
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Enter, &tx);
    a.handle_key(Key::Enter, &tx);
    a.handle_key(Key::Enter, &tx);
    let saved = std::fs::read_to_string(&config).unwrap();
    assert!(saved.contains("deepseek-custom") && saved.contains("deepseek-pro"), "{saved}");
    assert_eq!(crate::providers::Provider::DeepSeek.profile().model, "deepseek-flash", "the built-in changed");
    let (tx2, _rx2) = channel();
    a.run_command("provider", &tx2);
    assert!(open_picker(&a).items.iter().any(|i| i.value == "deepseek"), "the built-in left the picker");
}

#[test]
fn a_built_in_name_is_refused_for_a_new_provider() {
    let (mut a, config) = provider_app();
    let (tx, _rx) = channel();
    a.run_command("provider add openai", &tx);
    assert!(a.pending.is_none(), "a form opened for a built-in name");
    let shown = strip(&a.frame(90, 20));
    assert!(shown.contains("built-in"), "the refusal is not shown: {shown:?}");
    assert!(!std::fs::read_to_string(&config).unwrap().contains("openai"), "the name was saved");
}

#[test]
fn the_provider_footer_offers_use_and_configure() {
    let (mut a, _config) = provider_app();
    let (tx, _rx) = channel();
    a.run_command("provider", &tx);
    provider_row(&mut a, "ollama");
    let shown = strip(&a.frame(100, 30));
    assert!(shown.contains("u use") && shown.contains("e configure"), "footer: {shown:?}");
    assert_ne!(a.cfg.active_name, "ollama", "showing the footer switched the provider");
}

#[test]
fn the_settings_modal_lists_the_four_enumerable_rows_and_the_key() {
    let mut a = app();
    let (tx, _rx) = channel();
    a.run_command("settings", &tx);
    let keys: Vec<String> = open_picker(&a).items.iter().map(|i| i.value.clone()).collect();
    for key in ["allow_sudo", "color", "links", "title_summary", "api_key"] {
        assert!(keys.iter().any(|k| k == key), "no {key} row: {keys:?}");
    }
}

#[test]
fn enter_steps_color_and_saves_it_to_the_file() {
    let dir = std::env::temp_dir().join(format!("imp-sf-color-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config");
    std::fs::write(&path, "# keep\n").unwrap();
    let mut a = app();
    a.cfg.file = Some(path.clone());
    a.cfg.color = crate::config::ColorMode::Auto;
    let (tx, _rx) = channel();
    a.run_command("settings", &tx);
    focus_setting(&mut a, "color");
    a.handle_key(Key::Enter, &tx);
    assert_eq!(a.cfg.color, crate::config::ColorMode::Always);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "# keep\ncolor = always\n");
    let _ = std::fs::remove_dir_all(&dir);
}

fn file_config(name: &str, text: &str) -> (App, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("imp-sf-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config");
    std::fs::write(&path, text).unwrap();
    let mut a = app();
    a.cfg.file = Some(path.clone());
    (a, path)
}

#[test]
fn a_number_row_opens_a_form_that_saves_a_valid_value_and_keeps_comments() {
    let (mut a, path) = file_config("tokens", "# keep\n");
    let (tx, _rx) = channel();
    a.run_command("settings", &tx);
    focus_setting(&mut a, "max_tokens");
    a.handle_key(Key::Enter, &tx);
    assert!(matches!(a.pending, Some(Pending::SettingField { .. })), "no form opened");
    type_text(&mut a, &tx, "4096");
    a.handle_key(Key::Enter, &tx);
    assert_eq!(a.cfg.max_tokens, 4096, "the value did not apply");
    assert!(matches!(a.pending, Some(Pending::Pick(_))), "the modal did not return");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "# keep\nmax_tokens = 4096\n");
}

#[test]
fn a_refused_number_keeps_the_form_open_with_the_reason() {
    let (mut a, path) = file_config("steps", "# keep\n");
    let (tx, _rx) = channel();
    a.run_command("settings", &tx);
    focus_setting(&mut a, "max_steps");
    a.handle_key(Key::Enter, &tx);
    type_text(&mut a, &tx, "0");
    a.handle_key(Key::Enter, &tx);
    match &a.pending {
        Some(Pending::SettingField { error, .. }) => assert!(!error.is_empty(), "no reason shown"),
        _ => panic!("the form closed on a refused value"),
    }
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "# keep\n", "a refused value was saved");
}

#[test]
fn esc_on_a_text_form_saves_nothing() {
    let (mut a, path) = file_config("cancel", "# keep\nmodel = old\n");
    a.cfg.model = "old".into();
    let (tx, _rx) = channel();
    a.run_command("settings", &tx);
    focus_setting(&mut a, "model");
    a.handle_key(Key::Enter, &tx);
    type_text(&mut a, &tx, "x");
    a.handle_key(Key::Esc, &tx);
    assert!(matches!(a.pending, Some(Pending::Pick(_))), "Esc did not return to the modal");
    assert_eq!(a.cfg.model, "old");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "# keep\nmodel = old\n");
}

#[test]
fn the_api_key_row_shows_set_or_not_set_and_never_the_value() {
    let mut a = app();
    a.cfg.api_key = format!("sk-{}", "secret-value-123");
    let (tx, _rx) = channel();
    a.run_command("settings", &tx);
    let rows = settings_row_values(&a);
    let desc = rows.iter().find(|(k, _)| k == "api_key").map(|(_, d)| d.clone()).unwrap();
    assert!(desc.contains("set") && !desc.contains("sk-"), "{desc}");
    let shown = strip(&a.frame(100, 30));
    assert!(!shown.contains("sk-secret"), "the key was drawn: {shown:?}");
}

#[test]
fn the_settings_footer_names_the_config_file_for_grouped_settings() {
    let mut a = app();
    let (tx, _rx) = channel();
    a.run_command("settings", &tx);
    let shown = strip(&a.frame(120, 30));
    assert!(shown.contains("config file"), "the footer does not name the file: {shown:?}");
}

#[test]
fn the_api_key_row_opens_a_masked_field_that_never_draws_the_typed_key() {
    let mut a = app();
    let (tx, _rx) = channel();
    a.run_command("settings", &tx);
    focus_setting(&mut a, "api_key");
    a.handle_key(Key::Enter, &tx);
    assert!(matches!(a.pending, Some(Pending::KeyEntry { .. })), "the key row did not open the key entry");
    type_text(&mut a, &tx, "typed-secret-abc");
    let shown = strip(&a.frame(100, 30));
    assert!(!shown.contains("typed-secret-abc"), "the typed key was drawn: {shown:?}");
    assert!(!shown.contains("typed-secret"), "part of the typed key was drawn: {shown:?}");
}

#[test]
fn e_on_the_custom_provider_row_shows_the_form() {
    let (mut a, _config) = provider_app();
    a.cfg.named.push(crate::config::NamedProvider {
        name: "work".into(),
        base: "http://box/v1".into(),
        model: "m1".into(),
        key_env: String::new(),
        dialect: "openai".into(),
        models: Vec::new(),
    });
    let (tx, _rx) = channel();
    a.run_command("provider", &tx);
    provider_row(&mut a, "work");
    a.handle_key(Key::Char('e'), &tx);
    let shown = strip(&a.frame(100, 30));
    assert!(shown.contains("provider work"), "the form is not on screen: {shown:?}");
}

#[test]
fn e_on_the_built_in_custom_row_shows_the_form_at_a_narrow_width() {
    let (mut a, _config) = provider_app();
    let (tx, _rx) = channel();
    a.run_command("provider", &tx);
    provider_row(&mut a, "custom");
    a.handle_key(Key::Char('e'), &tx);
    let shown = strip(&a.frame(60, 20));
    assert!(shown.contains("provider custom-custom"), "the form is not on screen: {shown:?}");
}

#[test]
fn ctrl_e_configures_the_row_after_slash_provider() {
    let (mut a, _config) = provider_app();
    a.cfg.named.push(crate::config::NamedProvider {
        name: "work".into(),
        base: "http://box/v1".into(),
        model: "m1".into(),
        key_env: String::new(),
        dialect: "openai".into(),
        models: Vec::new(),
    });
    let (tx, _rx) = channel();
    a.input = "/provider ".into();
    a.cursor = a.input.len();
    a.sync_menu();
    provider_row(&mut a, "work");
    a.handle_key(Key::Ctrl('e'), &tx);
    assert!(matches!(a.pending, Some(Pending::ProviderForm { .. })), "Ctrl+E did not open the form: input {:?}", a.input);
    assert_eq!(a.input, "/provider ", "a letter reached the composer");
}

#[test]
fn the_provider_footer_names_the_key_for_the_way_the_picker_opened() {
    let (mut a, _config) = provider_app();
    let (tx, _rx) = channel();
    a.run_command("provider", &tx);
    provider_row(&mut a, "ollama");
    assert!(strip(&a.frame(100, 30)).contains("e configure"), "bare picker footer");
    let (mut b, _config) = provider_app();
    b.input = "/provider ".into();
    b.cursor = b.input.len();
    b.sync_menu();
    provider_row(&mut b, "ollama");
    let shown = strip(&b.frame(100, 30));
    assert!(shown.contains("^E configure"), "typed picker footer: {shown:?}");
    assert!(!shown.contains(" e configure"), "typed picker offers e: {shown:?}");
}

fn form_field(a: &App, label: &str) -> String {
    match &a.pending {
        Some(Pending::ProviderForm { fields, .. }) => {
            fields.iter().find(|(l, _)| l == label).map(|(_, v)| v.clone()).unwrap_or_default()
        }
        _ => panic!("no provider form is open"),
    }
}

fn form_at(a: &App) -> usize {
    match &a.pending {
        Some(Pending::ProviderForm { at, .. }) => *at,
        _ => panic!("no provider form is open"),
    }
}

fn open_add_form(a: &mut App, tx: &Sender<Cmd>) {
    a.run_command("provider add work", tx);
    assert!(matches!(a.pending, Some(Pending::ProviderForm { .. })));
}

#[test]
fn the_dialect_steps_through_the_six_values() {
    let (mut a, _config) = provider_app();
    let (tx, _rx) = channel();
    open_add_form(&mut a, &tx);
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Down, &tx);
    assert_eq!(form_at(&a), 2);
    a.handle_key(Key::Right, &tx);
    assert_eq!(form_field(&a, "dialect"), "deepseek");
    a.handle_key(Key::Right, &tx);
    assert_eq!(form_field(&a, "dialect"), "openai");
    a.handle_key(Key::Left, &tx);
    assert_eq!(form_field(&a, "dialect"), "deepseek");
    a.handle_key(Key::Left, &tx);
    assert_eq!(form_field(&a, "dialect"), "custom", "Left did not wrap");
    a.handle_key(Key::Char(' '), &tx);
    assert_eq!(form_field(&a, "dialect"), "deepseek", "Space did not step");
}

#[test]
fn typed_text_is_ignored_in_the_dialect() {
    let (mut a, _config) = provider_app();
    let (tx, _rx) = channel();
    open_add_form(&mut a, &tx);
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Right, &tx);
    let before = form_field(&a, "dialect");
    type_text(&mut a, &tx, "xyz");
    a.handle_key(Key::Backspace, &tx);
    assert_eq!(form_field(&a, "dialect"), before, "typed text reached the dialect");
}

#[test]
fn diffs_go_side_by_side_from_120_columns_and_cut_long_cells() {
    assert!(!side_by_side(119), "119 columns must stay inline");
    assert!(side_by_side(120));
    assert_eq!(fit_cell("abcdefghij", 5), "abcd…");
    assert_eq!(fit_cell("ab", 5), "ab   ");
    let row = sbs_line("old", "new", 120);
    assert_eq!(row.chars().count(), 120, "{row:?}");
    assert!(row.contains(" │ "), "{row:?}");
}

#[test]
fn a_wide_edit_draws_its_change_side_by_side() {
    let mut a = app();
    a.push_diffs(vec!["-let x = 1;".to_string(), "+let x = 2;".to_string()], "rs");
    let wide = strip(&a.frame(120, 20));
    assert!(
        wide.lines().any(|l| l.contains("let x = 1;") && l.contains(" │ ") && l.contains("let x = 2;")),
        "{wide}"
    );
    let narrow = strip(&a.frame(100, 20));
    assert!(!narrow.lines().any(|l| l.contains(" │ ") && l.contains("let x = 2;")), "{narrow}");
}

#[test]
fn skills_picker_toggles_and_saves_a_skill() {
    let dir = std::env::temp_dir().join(format!("imp-skills-picker-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let config = dir.join("config");
    std::fs::write(&config, "").unwrap();
    let mut a = app();
    a.config_path = config.clone();
    let (tx, _cmds) = channel();
    a.run_command("skills", &tx);
    let picker = open_picker(&a);
    assert_eq!(picker.kind, PickKind::Skill);
    assert!(picker.items.iter().any(|i| i.value == "config"), "{:?}", picker.items.len());
    a.handle_key(Key::Enter, &tx);
    assert!(a.cfg.skill_disabled("config"));
    assert!(std::fs::read_to_string(&config).unwrap().contains("skill.config.enabled = false"));
    let loaded = |a: &App| crate::store::load_skills(&a.cfg, &a.root);
    assert!(!loaded(&a).iter().any(|s| s.name == "config"), "a switched-off skill is still loaded");
    // Choosing it again turns it back on.
    a.run_command("skills", &tx);
    a.handle_key(Key::Enter, &tx);
    assert!(!a.cfg.skill_disabled("config"));
    assert!(loaded(&a).iter().any(|s| s.name == "config"));
}

#[test]
fn an_unconfigured_or_disabled_provider_is_not_in_the_model_list() {
    let (mut a, _config) = provider_app();
    // Unconfigured: the row is offered only when this process has the key.
    let keyed = std::env::var("OPENAI_API_KEY").is_ok_and(|v| !v.trim().is_empty());
    let values: Vec<String> = a.model_items().into_iter().map(|i| i.value).collect();
    assert_eq!(values.iter().any(|v| v.starts_with("openai::")), keyed, "{values:?}");
    // A disabled provider is left out.
    a.cfg.set("provider.openai.enabled", "false").unwrap();
    let values: Vec<String> = a.model_items().into_iter().map(|i| i.value).collect();
    assert!(!values.iter().any(|v| v.starts_with("openai::")), "{values:?}");
}

#[test]
fn an_editor_target_must_exist_inside_the_session_directory() {
    let root = std::env::temp_dir().join(format!("imp-target-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("sub")).unwrap();
    std::fs::write(root.join("sub/notes.md"), "x").unwrap();
    assert!(check_target(&root, "sub/notes.md").is_ok());
    assert!(check_target(&root, "sub").is_ok());
    let missing = check_target(&root, "new.md").unwrap_err();
    assert!(missing.contains("does not exist"), "{missing}");
    let outside = check_target(&root, "..").unwrap_err();
    assert!(outside.contains("outside the session directory"), "{outside}");
}

#[test]
fn an_editor_command_gets_the_target_as_one_argument() {
    let cfg = Config::default();
    let argv = editor_argv_for(&cfg, |v| (v == "EDITOR").then(|| "vim".to_string()), false, "my notes.md").unwrap();
    assert_eq!(argv, vec!["vim".to_string(), "my notes.md".to_string()]);
    assert!(editor_argv_for(&cfg, |_| None, false, "x").is_err(), "no editor must say so");
    let open = editor_argv_for(&cfg, |_| None, true, "x").unwrap();
    let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    assert_eq!(open, vec![opener.to_string(), "x".to_string()]);
}

#[test]
fn the_providers_list_marks_unconfigured_and_disabled_rows() {
    let (mut a, _config) = provider_app();
    a.cfg.set("provider.anthropic.enabled", "false").unwrap();
    let items = a.provider_items();
    let row = |v: &str| items.iter().find(|i| i.value == v).unwrap_or_else(|| panic!("no {v} row"));
    // Anthropic is switched off: it says so, whatever its key.
    assert!(row("anthropic").desc.contains("off"), "{:?}", row("anthropic").desc);
    // OpenAI with no key is marked unconfigured, unless this process has one.
    let keyed = std::env::var("OPENAI_API_KEY").is_ok_and(|v| !v.trim().is_empty());
    if !keyed {
        assert!(row("openai").desc.contains("no key"), "{:?}", row("openai").desc);
    }
    // The active provider keeps the current marker.
    assert!(row("deepseek").desc.contains("current"), "{:?}", row("deepseek").desc);
}

#[test]
fn the_active_provider_stays_in_the_model_list_even_unconfigured() {
    let (mut a, _config) = provider_app();
    a.cfg.set_active("openai").unwrap();
    a.cfg.set("provider.openai.enabled", "false").unwrap();
    let values: Vec<String> = a.model_items().into_iter().map(|i| i.value).collect();
    assert!(values.iter().any(|v| v.starts_with("openai::")), "active provider hidden: {values:?}");
}

#[test]
fn every_model_row_names_its_provider() {
    let (mut a, _config) = provider_app();
    let recents = a.config_path.parent().unwrap().join("models");
    std::fs::write(&recents, "deepseek-pro\n").unwrap();
    a.models_path = recents;
    a.available_models = vec!["deepseek-listed".to_string()];
    let bare: Vec<String> = a
        .model_items()
        .into_iter()
        .map(|i| i.value)
        .filter(|v| !v.contains("::"))
        .collect();
    assert!(bare.is_empty(), "rows with no provider: {bare:?}");
}

#[test]
fn tab_on_the_model_opens_the_default_recents_and_type_another() {
    let (mut a, _config) = provider_app();
    let recents = a.config_path.parent().unwrap().join("models");
    std::fs::write(&recents, "deepseek-pro\n").unwrap();
    a.models_path = recents;
    let (tx, _rx) = channel();
    open_add_form(&mut a, &tx);
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Right, &tx);
    a.handle_key(Key::Up, &tx);
    a.handle_key(Key::Tab, &tx);
    let picker = match &a.pending {
        Some(Pending::Pick(p)) if p.kind == PickKind::ProviderModel => p,
        _ => panic!("Tab on the model did not open the model picker"),
    };
    let values: Vec<String> = picker.items.iter().map(|i| i.value.clone()).collect();
    assert_eq!(values.first().map(String::as_str), Some("deepseek-flash"), "{values:?}");
    assert!(values.iter().any(|v| v == "deepseek-pro"), "no recent model: {values:?}");
    assert_eq!(values.last().map(String::as_str), Some("::type-another"), "{values:?}");
}

#[test]
fn choosing_a_recent_model_fills_the_field_and_returns_to_the_form() {
    let (mut a, _config) = provider_app();
    let recents = a.config_path.parent().unwrap().join("models2");
    std::fs::write(&recents, "deepseek-pro\n").unwrap();
    a.models_path = recents;
    let (tx, _rx) = channel();
    open_add_form(&mut a, &tx);
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Right, &tx);
    a.handle_key(Key::Up, &tx);
    a.handle_key(Key::Tab, &tx);
    provider_row(&mut a, "deepseek-pro");
    a.handle_key(Key::Enter, &tx);
    assert!(matches!(a.pending, Some(Pending::ProviderForm { .. })), "the form did not return");
    assert_eq!(form_field(&a, "model"), "deepseek-pro");
    assert_eq!(form_at(&a), 1);
}

#[test]
fn type_another_and_esc_return_to_the_form_with_the_text_unchanged() {
    let (mut a, _config) = provider_app();
    let (tx, _rx) = channel();
    open_add_form(&mut a, &tx);
    a.handle_key(Key::Down, &tx);
    type_text(&mut a, &tx, "kept");
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Right, &tx);
    a.handle_key(Key::Up, &tx);
    a.handle_key(Key::Tab, &tx);
    a.handle_key(Key::Esc, &tx);
    assert!(matches!(a.pending, Some(Pending::ProviderForm { .. })), "Esc did not return to the form");
    assert_eq!(form_field(&a, "model"), "kept");
    a.handle_key(Key::Tab, &tx);
    provider_row(&mut a, "::type-another");
    a.handle_key(Key::Enter, &tx);
    assert!(matches!(a.pending, Some(Pending::ProviderForm { .. })), "type another did not return");
    assert_eq!(form_field(&a, "model"), "kept");
}

#[test]
fn the_model_field_still_takes_typed_names() {
    let (mut a, _config) = provider_app();
    let (tx, _rx) = channel();
    open_add_form(&mut a, &tx);
    a.handle_key(Key::Down, &tx);
    type_text(&mut a, &tx, "my-model-7");
    assert_eq!(form_field(&a, "model"), "my-model-7");
    assert!(matches!(a.pending, Some(Pending::ProviderForm { .. })), "typing opened a picker");
}

#[test]
fn the_provider_form_shows_every_field_in_a_box() {
    let (mut a, _config) = provider_app();
    let (tx, _rx) = channel();
    a.run_command("provider add work", &tx);
    let shown = strip(&a.frame(80, 24));
    for label in ["base", "model", "dialect", "key_env"] {
        assert!(shown.split("\r\n").any(|row| row.contains(label)), "no {label} row on screen: {shown:?}");
    }
    assert!(shown.contains("╭─ provider work"), "the form has no box: {shown:?}");
    assert!(shown.contains("Esc cancel"), "the form has no keys line: {shown:?}");
}



#[test]
fn enter_on_a_provider_row_opens_its_configuration_form() {
    let (mut a, _config) = provider_app();
    let (tx, _rx) = channel();
    a.run_command("provider", &tx);
    provider_row(&mut a, "ollama");
    a.handle_key(Key::Enter, &tx);
    match &a.pending {
        Some(Pending::ProviderForm { name, fields, .. }) => {
            assert_eq!(name, "ollama-custom");
            assert_eq!(fields[0].1, crate::providers::Provider::Ollama.profile().base_url);
        }
        _ => panic!("Enter did not open the configuration form"),
    }
    assert_ne!(a.cfg.active_name, "ollama", "Enter switched the provider");
}

#[test]
fn u_on_a_provider_row_switches_to_it() {
    let (mut a, _config) = provider_app();
    let (tx, _rx) = channel();
    a.run_command("provider", &tx);
    provider_row(&mut a, "ollama");
    let shown = strip(&a.frame(100, 30));
    assert!(shown.contains("u use"), "the footer has no use hint: {shown:?}");
    a.handle_key(Key::Char('u'), &tx);
    assert_eq!(a.cfg.active_name, "ollama", "u did not switch");
    assert!(a.pending.is_none(), "the picker stayed open");
}

#[test]
fn ctrl_u_switches_from_a_typed_provider_picker() {
    let (mut a, _config) = provider_app();
    let (tx, _rx) = channel();
    a.input = "/provider ".into();
    a.cursor = a.input.len();
    a.sync_menu();
    provider_row(&mut a, "ollama");
    let shown = strip(&a.frame(100, 30));
    assert!(shown.contains("^U use"), "the typed footer has no use hint: {shown:?}");
    a.handle_key(Key::Ctrl('u'), &tx);
    assert_eq!(a.cfg.active_name, "ollama", "Ctrl+U did not switch");
    assert_eq!(a.input, "/provider ", "a letter reached the composer");
}

fn settings_row_values(a: &App) -> Vec<(String, String)> {
    open_picker(a).items.iter().map(|i| (i.value.clone(), i.desc.clone())).collect()
}

#[test]
fn typing_edits_a_provider_field_in_the_box() {
    let (mut a, _config) = provider_app();
    let (tx, _rx) = channel();
    a.run_command("provider add work", &tx);
    type_text(&mut a, &tx, "http://x");
    assert_eq!(form_field(&a, "base"), "http://x", "typing did not edit base");
    let shown = strip(&a.frame(80, 24));
    assert!(shown.contains("http://x"), "the edited base is not on screen");
}

#[test]
fn the_provider_form_has_an_api_key_field_that_is_masked() {
    let (mut a, _config) = provider_app();
    let (tx, _rx) = channel();
    open_add_form(&mut a, &tx);
    assert_eq!(form_field(&a, "api_key"), "", "the key field is missing");
    for _ in 0..5 {
        a.handle_key(Key::Down, &tx);
    }
    type_text(&mut a, &tx, "sk-typed-key-9");
    assert_eq!(form_field(&a, "api_key"), "sk-typed-key-9");
    let shown = strip(&a.frame(80, 24));
    assert!(!shown.contains("sk-typed-key-9"), "the typed key was drawn: {shown:?}");
    assert!(shown.contains("•••"), "the key is not shown as masked dots");
}

#[test]
fn saving_with_a_key_but_no_variable_name_is_refused_and_nothing_is_saved() {
    let (mut a, config) = provider_app();
    let (tx, _rx) = channel();
    open_add_form(&mut a, &tx);
    for (i, text) in ["http://box/v1", "m1"].iter().enumerate() {
        if i > 0 {
            a.handle_key(Key::Down, &tx);
        }
        type_text(&mut a, &tx, text);
    }
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Right, &tx);
    a.handle_key(Key::Right, &tx);
    // key_env stays empty; down past models to the key row.
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Down, &tx);
    type_text(&mut a, &tx, "sk-no-variable-1");
    a.handle_key(Key::Enter, &tx);
    assert!(matches!(a.pending, Some(Pending::ProviderForm { .. })), "the form closed");
    let shown = strip(&a.frame(90, 24));
    assert!(shown.contains("key variable"), "no reason shown: {shown:?}");
    assert!(!std::fs::read_to_string(&config).unwrap().contains("sk-no-variable-1"), "the key reached the config");
}

#[test]
fn an_input_edits_at_the_caret_and_shows_it() {
    let (mut a, _config) = provider_app();
    let (tx, _rx) = channel();
    open_add_form(&mut a, &tx);
    type_text(&mut a, &tx, "abc");
    a.handle_key(Key::Left, &tx);
    type_text(&mut a, &tx, "X");
    assert_eq!(form_field(&a, "base"), "abXc", "text did not go in at the caret");
    a.handle_key(Key::Left, &tx);
    a.handle_key(Key::Left, &tx);
    a.handle_key(Key::Backspace, &tx);
    assert_eq!(form_field(&a, "base"), "bXc", "backspace did not delete before the caret");
    let shown = strip(&a.frame(80, 24));
    assert!(shown.contains("▏bXc"), "the caret is not drawn in the active input: {shown:?}");
}

#[test]
fn the_active_select_shows_its_arrows() {
    let (mut a, _config) = provider_app();
    let (tx, _rx) = channel();
    open_add_form(&mut a, &tx);
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Down, &tx);
    a.handle_key(Key::Right, &tx);
    let shown = strip(&a.frame(80, 24));
    assert!(shown.contains("‹ deepseek ›"), "the dialect select has no arrows: {shown:?}");
}


#[test]
fn saving_without_a_model_says_which_fields_are_required() {
    let (mut a, _config) = provider_app();
    let (tx, _rx) = channel();
    a.run_command("provider add work", &tx);
    type_text(&mut a, &tx, "http://x");
    a.handle_key(Key::Enter, &tx);
    a.handle_key(Key::Enter, &tx);
    a.handle_key(Key::Enter, &tx);
    a.handle_key(Key::Enter, &tx);
    a.handle_key(Key::Enter, &tx);
    a.handle_key(Key::Enter, &tx);
    let shown = strip(&a.frame(90, 24));
    assert!(shown.contains("required"), "no required message: {shown:?}");
}

#[test]
fn the_models_field_saves_a_comma_separated_list() {
    let (mut a, config) = provider_app();
    let (tx, _rx) = channel();
    open_add_form(&mut a, &tx);
    for text in ["http://box/v1", "m1"] {
        type_text(&mut a, &tx, text);
        a.handle_key(Key::Enter, &tx);
    }
    a.handle_key(Key::Right, &tx);
    a.handle_key(Key::Right, &tx);
    a.handle_key(Key::Enter, &tx);
    a.handle_key(Key::Enter, &tx);
    type_text(&mut a, &tx, "m1, m2");
    assert_eq!(form_field(&a, "models"), "m1, m2", "the models field did not take text");
    a.handle_key(Key::Enter, &tx);
    a.handle_key(Key::Enter, &tx);
    let saved = std::fs::read_to_string(&config).unwrap();
    assert!(saved.contains("provider.work.models = m1, m2"), "{saved}");
}

fn work_provider(a: &mut App) {
    a.cfg.named.push(crate::config::NamedProvider {
        name: "work".into(),
        base: "http://box/v1".into(),
        model: "m1".into(),
        models: vec!["m2".into()],
        key_env: String::new(),
        dialect: "openai".into(),
    });
}

#[test]
fn the_model_picker_lists_every_configured_providers_models() {
    let (mut a, _config) = provider_app();
    work_provider(&mut a);
    let (tx, _rx) = channel();
    a.run_command("model", &tx);
    let values: Vec<String> = open_picker(&a).items.iter().map(|i| i.value.clone()).collect();
    assert!(values.iter().any(|v| v == "work::m1"), "no work::m1 row: {values:?}");
    assert!(values.iter().any(|v| v == "work::m2"), "no work::m2 row: {values:?}");
    assert!(values.iter().any(|v| v == "deepseek::deepseek-flash"), "no built-in row: {values:?}");
}

#[test]
fn choosing_a_model_from_another_provider_switches_both() {
    let (mut a, _config) = provider_app();
    work_provider(&mut a);
    let (tx, _rx) = channel();
    a.run_command("model", &tx);
    provider_row(&mut a, "work::m2");
    a.handle_key(Key::Enter, &tx);
    assert_eq!(a.cfg.active_name, "work", "the provider did not switch");
    assert_eq!(a.cfg.model, "m2", "the model did not switch");
}

#[test]
fn the_status_line_names_a_configured_provider() {
    let (mut a, _config) = provider_app();
    work_provider(&mut a);
    let (tx, _rx) = channel();
    a.run_command("model", &tx);
    provider_row(&mut a, "work::m2");
    a.handle_key(Key::Enter, &tx);
    // The "model: m2" toast takes the status line for three seconds; the provider shows after it.
    a.status.toast = None;
    let line = a.status_line(120);
    assert!(line.contains("m2 (work)"), "the status line does not name the provider: {line}");
}

fn goal_app() -> (App, Sender<Cmd>, Receiver<Cmd>) {
    let (mut a, _config) = provider_app();
    let (tx, rx) = channel::<Cmd>();
    let (_ev_tx, ev_rx) = channel::<Ev>();
    a.worker = Some((tx.clone(), ev_rx, std::sync::Arc::new(std::sync::Mutex::new(String::new()))));
    (a, tx, rx)
}

#[test]
fn goal_shows_its_text_round_and_state() {
    let (mut a, tx, _rx) = goal_app();
    a.status.goal = Some(("fix the parser".into(), 2, 5));
    a.status.goal_paused = true;
    a.run_command("goal", &tx);
    let shown = strip(&a.frame(100, 30));
    assert!(shown.contains("fix the parser") && shown.contains("round 2/5") && shown.contains("paused"), "{shown:?}");
}

#[test]
fn goal_pause_resume_clear_and_edit_send_their_commands() {
    let (mut a, tx, rx) = goal_app();
    a.status.goal = Some(("fix the parser".into(), 2, 5));
    a.run_command("goal pause", &tx);
    assert!(matches!(rx.try_recv(), Ok(Cmd::GoalPause)), "pause was not sent");
    a.run_command("goal resume", &tx);
    assert!(matches!(rx.try_recv(), Ok(Cmd::Goal(t)) if t.is_empty()), "resume was not sent");
    a.run_command("goal clear", &tx);
    assert!(matches!(rx.try_recv(), Ok(Cmd::GoalOff)), "clear was not sent");
    a.run_command("goal edit fix the lexer", &tx);
    assert!(matches!(rx.try_recv(), Ok(Cmd::GoalEdit(t)) if t == "fix the lexer"), "edit was not sent");
}

#[test]
fn a_new_goal_asks_before_replacing_a_running_one() {
    let (mut a, tx, rx) = goal_app();
    a.status.goal = Some(("fix the parser".into(), 2, 5));
    a.run_command("goal write tests", &tx);
    assert!(matches!(a.pending, Some(Pending::GoalReplace { .. })), "no replace question");
    assert!(rx.try_recv().is_err(), "the new goal started before the answer");
    a.handle_key(Key::Char('n'), &tx);
    assert!(a.pending.is_none());
    assert!(rx.try_recv().is_err(), "declining still changed the goal");
    a.run_command("goal write tests", &tx);
    a.handle_key(Key::Char('y'), &tx);
    assert!(matches!(rx.try_recv(), Ok(Cmd::Goal(t)) if t == "write tests"), "replace was not sent");
}

#[test]
fn the_status_line_shows_a_paused_goal() {
    let (mut a, _tx, _rx) = goal_app();
    a.status.goal = Some(("fix the parser".into(), 2, 5));
    a.status.goal_paused = true;
    let line = a.status_line(120);
    assert!(line.contains("goal 2/5 · paused"), "{line}");
}

#[test]
fn typing_after_opening_a_saved_provider_edits_at_the_end() {
    let (mut a, _config) = provider_app();
    a.cfg.named.push(crate::config::NamedProvider {
        name: "work".into(),
        base: "http://box/v1".into(),
        model: "m1".into(),
        models: Vec::new(),
        key_env: String::new(),
        dialect: "openai".into(),
    });
    let (tx, _rx) = channel();
    a.run_command("provider", &tx);
    provider_row(&mut a, "work");
    a.handle_key(Key::Enter, &tx);
    type_text(&mut a, &tx, "X");
    assert_eq!(form_field(&a, "base"), "http://box/v1X", "typing did not append to the saved value");
}

#[test]
fn typing_after_opening_the_add_form_goes_into_the_first_field() {
    let (mut a, _config) = provider_app();
    let (tx, _rx) = channel();
    open_add_form(&mut a, &tx);
    type_text(&mut a, &tx, "abc");
    assert_eq!(form_field(&a, "base"), "abc");
}

#[test]
fn enter_on_custom_opens_the_provider_editor_and_keeps_the_provider() {
    let mut a = app();
    a.config_path = scratch("provider-picker-enter").join("config");
    let (tx, _rx) = channel();
    a.run_command("provider", &tx);
    let at = open_picker(&a).items.iter().position(|i| i.value == "custom").expect("custom row");
    for _ in 0..at {
        a.handle_key(Key::Down, &tx);
    }
    let picker = open_picker(&a);
    assert_eq!(picker.items[picker.index].value, "custom");
    let before = a.cfg.active_name.clone();
    a.handle_key(Key::Enter, &tx);
    assert!(
        matches!(a.pending, Some(Pending::ProviderForm { .. })),
        "Enter on custom did not open the provider editor"
    );
    assert_eq!(a.cfg.active_name, before, "Enter switched the active provider");
}

#[test]
fn u_still_switches_to_the_highlighted_provider() {
    let mut a = app();
    a.config_path = scratch("provider-picker-use").join("config");
    let (tx, _rx) = channel();
    a.run_command("provider", &tx);
    let at = open_picker(&a).items.iter().position(|i| i.value == "custom").expect("custom row");
    for _ in 0..at {
        a.handle_key(Key::Down, &tx);
    }
    a.handle_key(Key::Char('u'), &tx);
    assert_eq!(a.cfg.active_name, "custom");
}

#[test]
fn provider_rows_keep_their_height_as_the_selection_moves() {
    let a = app();
    let mut picker = a.value_picker("provider", a.provider_items(), PickKind::Provider);
    let matches = picker.matches();
    let visible = matches.len();
    let mut heights = Vec::new();
    for at in 0..visible {
        picker.index = at;
        heights.push(picker_lines(&picker, &matches, visible, 50, true).len());
    }
    assert!(
        heights.iter().all(|h| *h == heights[0]),
        "the list changed height as the selection moved: {heights:?}"
    );
}
