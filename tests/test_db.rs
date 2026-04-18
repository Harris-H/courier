use courier::db::Database;
use courier::scheduler::task::TaskStatus;
use courier::scheduler::ExecutionRecord;

fn make_record(name: &str, status: TaskStatus) -> ExecutionRecord {
    ExecutionRecord {
        task_name: name.to_string(),
        status,
        executed_at: chrono::Local::now(),
        completed_at: Some(chrono::Local::now()),
        duration_ms: 1234,
        articles_count: 5,
        error_message: None,
        digest_content: Some("Test digest".to_string()),
    }
}

#[test]
fn db_insert_and_retrieve_record() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().to_str().unwrap()).unwrap();

    let record = make_record("daily-digest", TaskStatus::Success);
    let id = db.insert_record(&record).unwrap();
    assert!(id > 0);

    let history = db.get_history(10).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].task_name, "daily-digest");
    assert_eq!(history[0].status, TaskStatus::Success);
    assert_eq!(history[0].articles_count, 5);
}

#[test]
fn db_update_running_to_success() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().to_str().unwrap()).unwrap();

    let running = make_record("task1", TaskStatus::Running);
    let id = db.insert_record(&running).unwrap();

    let mut completed = running.clone();
    completed.status = TaskStatus::Success;
    completed.duration_ms = 5000;
    completed.articles_count = 10;
    db.update_record(id, &completed).unwrap();

    let history = db.get_history(10).unwrap();
    assert_eq!(history[0].status, TaskStatus::Success);
    assert_eq!(history[0].duration_ms, 5000);
    assert_eq!(history[0].articles_count, 10);
}

#[test]
fn db_update_running_to_failed() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().to_str().unwrap()).unwrap();

    let running = make_record("task1", TaskStatus::Running);
    let id = db.insert_record(&running).unwrap();

    let failed = ExecutionRecord {
        task_name: "task1".to_string(),
        status: TaskStatus::Failed,
        executed_at: running.executed_at,
        completed_at: Some(chrono::Local::now()),
        duration_ms: 3000,
        articles_count: 0,
        error_message: Some("Network timeout".to_string()),
        digest_content: None,
    };
    db.update_record(id, &failed).unwrap();

    let history = db.get_history(10).unwrap();
    assert_eq!(history[0].status, TaskStatus::Failed);
    assert_eq!(history[0].error_message.as_deref(), Some("Network timeout"));
}

#[test]
fn db_get_history_respects_limit() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().to_str().unwrap()).unwrap();

    for i in 0..10 {
        let record = make_record(&format!("task-{}", i), TaskStatus::Success);
        db.insert_record(&record).unwrap();
    }

    let history = db.get_history(3).unwrap();
    assert_eq!(history.len(), 3);
}

#[test]
fn db_get_history_orders_by_newest_first() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().to_str().unwrap()).unwrap();

    db.insert_record(&make_record("first", TaskStatus::Success))
        .unwrap();
    db.insert_record(&make_record("second", TaskStatus::Success))
        .unwrap();
    db.insert_record(&make_record("third", TaskStatus::Success))
        .unwrap();

    let history = db.get_history(10).unwrap();
    assert_eq!(history[0].task_name, "third");
    assert_eq!(history[2].task_name, "first");
}

#[test]
fn db_clear_all_history_empties_table() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().to_str().unwrap()).unwrap();

    db.insert_record(&make_record("task", TaskStatus::Success))
        .unwrap();
    db.insert_record(&make_record("task2", TaskStatus::Failed))
        .unwrap();

    let deleted = db.clear_all_history().unwrap();
    assert_eq!(deleted, 2);

    let history = db.get_history(10).unwrap();
    assert!(history.is_empty());
}

#[test]
fn db_empty_history_returns_empty_vec() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().to_str().unwrap()).unwrap();
    let history = db.get_history(10).unwrap();
    assert!(history.is_empty());
}
