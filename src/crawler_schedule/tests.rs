use super::*;
use chrono::TimeZone;
fn schedule(cron: &str, timezone: &str) -> CrawlerSchedule {
    CrawlerSchedule {
        enabled: true,
        cron: cron.into(),
        timezone: timezone.into(),
        args: vec![],
    }
}
#[test]
fn standard_cron_weekdays_and_dom_dow_are_consistent() {
    let now = Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap();
    for cron in ["0 0 * * 0", "0 0 * * 7", "0 0 * * SUN"] {
        assert_eq!(
            next_runs_after(&schedule(cron, "UTC"), "UTC", now, 1).unwrap()[0],
            Utc.with_ymd_and_hms(2026, 11, 8, 0, 0, 0).unwrap()
        );
    }
    assert_eq!(
        next_runs_after(&schedule("0 0 15 * MON", "UTC"), "UTC", now, 1).unwrap()[0],
        Utc.with_ymd_and_hms(2026, 11, 9, 0, 0, 0).unwrap()
    );
}
#[test]
fn timezone_and_five_fields_are_validated() {
    for cron in [
        "0 0 * *",
        "0 0 0 * * *",
        "60 0 * * *",
        "@daily",
        "0 0 31 2 *",
        "0 0 * * ?",
    ] {
        assert!(
            validate_schedule("leetcode", &mut schedule(cron, "UTC"), "UTC").is_err(),
            "{cron}"
        );
    }
    assert!(validate_schedule("diag", &mut schedule("0 0 * * *", "UTC"), "UTC").is_err());
    assert!(validate_schedule("leetcode", &mut schedule("0 0 * * *", "Bad/Zone"), "UTC").is_err());
    assert!(validate_schedule("leetcode", &mut CrawlerSchedule::default(), "UTC").is_ok());
}
#[test]
fn timezone_inherits_global_default() {
    let now = Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap();
    assert_eq!(
        next_runs_after(&schedule("30 3 * * *", ""), "Asia/Taipei", now, 1).unwrap()[0],
        Utc.with_ymd_and_hms(2026, 11, 7, 19, 30, 0).unwrap()
    );
    assert_eq!(
        next_runs_after(&schedule("30 3 * * *", "UTC"), "Asia/Taipei", now, 1).unwrap()[0],
        Utc.with_ymd_and_hms(2026, 11, 7, 3, 30, 0).unwrap()
    );
}
#[test]
fn args_are_bound_to_source() {
    let mut s = schedule("0 0 * * *", "UTC");
    validate_schedule("spoj", &mut s, "UTC").unwrap();
    assert_eq!(s.args, ["--source", "spoj"]);
    assert!(validate_schedule("luogu", &mut s, "UTC").is_err());
    s.args = vec!["--bad-flag".into()];
    assert!(validate_schedule("leetcode", &mut s, "UTC").is_err());
}
#[test]
fn malformed_persisted_source_does_not_disable_other_sources() {
    let pool = crate::db::create_rw_pool(":memory:", 1, 1000);
    crate::db::ensure_app_settings_table(&pool);
    let mut schedules = default_schedules();
    schedules.insert("leetcode".into(), schedule("30 3 * * *", "UTC"));
    save_schedules(&pool, &schedules).unwrap();
    assert_eq!(
        load_schedules(&pool, "UTC").unwrap()["leetcode"],
        schedules["leetcode"]
    );
    crate::db::settings::set_setting(
        &pool,
        SETTING_KEY,
        r#"{"atcoder":5,"leetcode":{"enabled":true,"cron":"0 * * * *","args":[]}}"#,
    );
    let loaded = load_schedules(&pool, "UTC").unwrap();
    assert!(!loaded["atcoder"].enabled);
    assert!(loaded["leetcode"].enabled);
}
#[test]
fn daylight_saving_previews_are_monotonic() {
    for (month, day) in [(3, 7), (10, 31)] {
        let now = Utc.with_ymd_and_hms(2026, month, day, 0, 0, 0).unwrap();
        let next =
            next_runs_after(&schedule("30 2 * * *", "America/New_York"), "UTC", now, 5).unwrap();
        assert!(next.windows(2).all(|p| p[0] < p[1]));
        assert!(next[0] > now);
    }
}

#[test]
fn deadlines_retain_due_occurrences_and_recalculate_only_changed_schedules() {
    let now = Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap();
    let mut deadlines = Deadlines::new();
    let mut schedules = default_schedules();
    schedules.insert("leetcode".into(), schedule("* * * * *", "UTC"));
    reconcile_deadlines(&mut deadlines, schedules.clone(), now, "UTC").unwrap();
    let first = deadlines["leetcode"].1;
    assert_eq!(first, now + chrono::Duration::minutes(1));
    reconcile_deadlines(
        &mut deadlines,
        schedules.clone(),
        now + chrono::Duration::minutes(2),
        "UTC",
    )
    .unwrap();
    assert_eq!(
        deadlines["leetcode"].1, first,
        "due jobs must not be lost on a clock jump or unrelated setting update"
    );
    schedules.get_mut("leetcode").unwrap().cron = "30 * * * *".into();
    reconcile_deadlines(&mut deadlines, schedules.clone(), now, "UTC").unwrap();
    assert_eq!(deadlines["leetcode"].1, now + chrono::Duration::minutes(30));
    schedules.get_mut("leetcode").unwrap().enabled = false;
    reconcile_deadlines(&mut deadlines, schedules, now, "UTC").unwrap();
    assert!(deadlines.is_empty());
}

#[test]
fn dst_fixed_time_runs_once_and_gap_moves_to_first_valid_time() {
    let s = schedule("30 1 * * *", "America/New_York");
    let now = Utc.with_ymd_and_hms(2026, 11, 1, 0, 0, 0).unwrap();
    let runs = next_runs_after(&s, "UTC", now, 2).unwrap();
    assert_eq!(
        runs[0],
        Utc.with_ymd_and_hms(2026, 11, 1, 5, 30, 0).unwrap()
    );
    assert_eq!(
        runs[1],
        Utc.with_ymd_and_hms(2026, 11, 2, 6, 30, 0).unwrap()
    );
    let s = schedule("30 2 * * *", "America/New_York");
    let now = Utc.with_ymd_and_hms(2026, 3, 8, 0, 0, 0).unwrap();
    assert_eq!(
        next_runs_after(&s, "UTC", now, 1).unwrap()[0],
        Utc.with_ymd_and_hms(2026, 3, 8, 7, 0, 0).unwrap()
    );
}

#[test]
fn cron_occurrences_have_zero_seconds_and_nanoseconds() {
    use chrono::Timelike;
    let runs = next_runs_after(&schedule("* * * * *", "UTC"), "UTC", Utc::now(), 5).unwrap();
    assert!(runs
        .iter()
        .all(|run| run.second() == 0 && run.nanosecond() == 0));
}
