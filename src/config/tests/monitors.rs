use super::{parse_err, parse_ok};
use crate::config::{Edge, ImageFit, Layer, Layout, ShowOn};

fn named(names: &[&str]) -> ShowOn {
    ShowOn::Named(names.iter().map(|name| (*name).to_owned()).collect())
}

fn show_on(raw: &str) -> (ShowOn, Vec<String>) {
    let parsed = parse_ok(raw);
    (parsed.config.overlay.show_on, parsed.warnings)
}

#[test]
fn show_on_takes_a_word_a_name_or_a_list() {
    let cases = [
        ("", ShowOn::Primary),
        ("[overlay]\nshow_on = \"primary\"\n", ShowOn::Primary),
        ("[overlay]\nshow_on = \"all\"\n", ShowOn::All),
        ("[overlay]\nshow_on = \"DP-2\"\n", named(&["DP-2"])),
        ("[overlay]\nshow_on = [\"DP-2\"]\n", named(&["DP-2"])),
        (
            "[overlay]\nshow_on = [\" DP-1 \", \"\", \"index:2\"]\n",
            named(&["DP-1", "index:2"]),
        ),
    ];
    for (raw, expected) in cases {
        assert_eq!(show_on(raw), (expected, Vec::new()), "{raw}");
    }
    let err = parse_err("[overlay]\nshow_on = 2\n");
    assert!(
        err.contains("line 2") && err.contains("a list of monitor names"),
        "{err}"
    );
}

#[test]
fn a_section_is_named_by_its_monitor() {
    let parsed = parse_ok(
        "[overlay]\nshow_on = [\"DP-2\"]\n\
         [output.DP-2.overlay]\nheight = 300\nposition = \"top\"\n\
         [output.DP-2.visualizer]\nlayout = \"wave\"\n\
         [output.\"index:3\"]\nenabled = false\n",
    );
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    let overlay = parsed.config.overlay;
    assert_eq!(overlay.show_on, named(&["DP-2"]));
    let [first, second] = overlay.outputs.as_slice() else {
        panic!("expected two sections, got {:?}", overlay.outputs);
    };
    assert_eq!(
        (first.monitor.as_str(), first.overlay.height),
        ("DP-2", Some(300))
    );
    assert_eq!(first.overlay.position, Some(Edge::Top));
    assert_eq!(first.visualizer.layout, Some(Layout::Wave));
    assert_eq!(
        (second.monitor.as_str(), second.enabled),
        ("index:3", false)
    );
}

#[test]
fn placement_keys_beside_the_overlay_table_still_work() {
    let parsed = parse_ok(
        "[output.DP-1]\nheight = 300\nposition = \"top\"\n\
         [output.DP-1.overlay]\nheight = 120\nlayer = \"top\"\n",
    );
    assert_eq!(
        parsed.warnings,
        vec!["output.DP-1.height: also set in [output.DP-1.overlay], which is used"]
    );
    let [section] = parsed.config.overlay.outputs.as_slice() else {
        panic!("expected one section");
    };
    let placement = &section.overlay;
    assert_eq!(
        (placement.height, placement.layer),
        (Some(120), Some(Layer::Top))
    );
    assert_eq!(placement.position, Some(Edge::Top));

    let legacy = parse_ok("[[overlay.outputs]]\nmonitor = \"DP-9\"\nwidth = 40\n");
    assert_eq!(legacy.config.overlay.outputs[0].overlay.width, Some(40));
}

#[test]
fn a_section_cannot_choose_monitors() {
    let parsed = parse_ok("[output.DP-1.overlay]\nshow_on = \"all\"\nfoo = 1\n");
    assert_eq!(
        parsed.warnings,
        vec![
            "output.DP-1.overlay.foo: unknown key, ignored",
            "output.DP-1.overlay.show_on: unknown key, ignored (monitors are chosen with `show_on` in the main [overlay])",
        ]
    );
}

#[test]
fn sections_without_show_on_choose_their_monitors() {
    let (chosen, warnings) = show_on("[output.DP-1]\nheight = 100\n");
    assert_eq!((chosen, warnings), (ShowOn::Sections, Vec::new()));
}

#[test]
fn section_mistakes_are_warnings() {
    let parsed = parse_ok("[output.DP-1]\nmonitor = \"DP-2\"\nfoo = 1\n");
    assert_eq!(parsed.config.overlay.outputs[0].monitor, "DP-1");
    assert_eq!(
        parsed.warnings,
        vec![
            "output.DP-1.foo: unknown key, ignored",
            "output.DP-1.monitor: the section name is the monitor, ignored",
        ]
    );
}

#[test]
fn the_old_selection_keys_still_work_and_name_their_replacement() {
    let cases = [
        (
            "monitor_mode = \"primary\"",
            ShowOn::Primary,
            "overlay.monitor_mode: deprecated, write `show_on = \"primary\"` instead",
        ),
        (
            "monitor_mode = \"all\"",
            ShowOn::All,
            "overlay.monitor_mode: deprecated, write `show_on = \"all\"` instead",
        ),
        (
            "monitor_mode = \"list\"\nmonitors = [\"DP-1\", \" HDMI-A-1 \", \"\"]",
            named(&["DP-1", "HDMI-A-1"]),
            "overlay.monitor_mode, overlay.monitors: deprecated, write `show_on = [\"DP-1\", \"HDMI-A-1\"]` instead",
        ),
    ];
    for (keys, expected, warning) in cases {
        let (chosen, warnings) = show_on(&format!("[overlay]\n{keys}\n"));
        assert_eq!(
            (chosen, warnings),
            (expected, vec![warning.to_owned()]),
            "{keys}"
        );
    }
}

#[test]
fn a_monitor_list_without_list_mode_says_that_it_does_nothing() {
    for keys in [
        "monitors = [\"DP-2\"]",
        "monitor_mode = \"primary\"\nmonitors = [\"DP-2\"]",
    ] {
        let (chosen, warnings) = show_on(&format!("[overlay]\n{keys}\n"));
        assert_eq!(chosen, ShowOn::Primary, "{keys}");
        assert_eq!(
            warnings,
            vec![
                "overlay.monitors has no effect without `monitor_mode = \"list\"`, the primary monitor is used; to show the bars on these monitors write `show_on = [\"DP-2\"]`"
            ],
            "{keys}"
        );
    }
}

#[test]
fn show_on_and_sections_win_over_the_old_keys() {
    let (chosen, warnings) =
        show_on("[overlay]\nshow_on = \"all\"\nmonitor_mode = \"list\"\nmonitors = [\"DP-1\"]\n");
    assert_eq!(chosen, ShowOn::All);
    assert_eq!(
        warnings,
        vec![
            "overlay.monitor_mode, overlay.monitors: deprecated and not used here, `show_on` or the [output.NAME] sections choose the monitors"
        ]
    );

    let (chosen, warnings) =
        show_on("[overlay]\nmonitor_mode = \"all\"\n[[overlay.outputs]]\nmonitor = \"DP-1\"\n");
    assert_eq!(chosen, ShowOn::Sections);
    assert_eq!(warnings.len(), 2, "{warnings:?}");
}

#[test]
fn keys_of_other_programs_point_at_the_right_one() {
    let parsed = parse_ok("[overlay]\noutput_mode = \"list\"\n[outputs.DP-2]\nheight = 3\n");
    assert_eq!(
        parsed.warnings,
        vec![
            "outputs: unknown key, ignored (per-monitor sections are written [output.NAME])",
            "overlay.output_mode: unknown key, ignored (to choose monitors write `show_on` in [overlay])",
        ]
    );

    let err = parse_err("[overlay]\noutputs = [\"DP-2\"]\n");
    assert!(err.contains("line 2"), "{err}");
    assert!(
        err.contains(
            "overlay.outputs does not take monitor names; to show the bars on \"DP-2\" write `show_on = [\"DP-2\"]` in [overlay]"
        ),
        "{err}"
    );
}

#[test]
fn a_section_can_change_the_image() {
    let parsed = parse_ok(
        "[image_overlay]\nenabled = true\npath = \"a.png\"\nopacity = 0.8\nwidth = 300\n\
         [output.DP-1.image_overlay]\npath = \" b.png \"\nfit = \"cover\"\noffset_y = -40\n\
         [output.DP-2.image_overlay]\nenabled = false\nopacity = 4.0\n\
         [output.DP-3]\nheight = 100\n",
    );
    assert_eq!(
        parsed.warnings,
        vec!["output.DP-2.image_overlay.opacity: 4 is outside 0..=1, using 1"]
    );
    let config = parsed.config;
    let [first, second, third] = config.overlay.outputs.as_slice() else {
        panic!("expected three sections");
    };
    let own = config.image(Some(first));
    assert_eq!(own.path.as_deref(), Some("b.png"));
    assert_eq!((own.fit, own.offset_y), (ImageFit::Cover, -40.0));
    // what the section leaves out comes from [image_overlay]
    assert_eq!((own.enabled, own.opacity, own.width), (true, 0.8, 300));
    assert!(!config.image(Some(second)).enabled);
    assert_eq!(config.image(Some(third)), config.image_overlay);
    assert_eq!(config.image(None), config.image_overlay);
}

#[test]
fn unknown_image_keys_in_a_section_are_named() {
    let parsed = parse_ok("[output.DP-1.image_overlay]\npth = \"a.png\"\n");
    assert_eq!(
        parsed.warnings,
        vec!["output.DP-1.image_overlay.pth: unknown key, ignored"]
    );
}
