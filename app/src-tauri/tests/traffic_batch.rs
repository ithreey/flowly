//! 批量查询事务详情测试

use flowly_gui::traffic::{SharedTraffic, should_capture_content_type};

#[test]
fn test_get_batch_returns_details_in_order() {
    let traffic = SharedTraffic::new();

    // 模拟 3 个事务
    traffic.begin_request(
        1,
        "GET".to_string(),
        "http://example.com/1".to_string(),
        "example.com".to_string(),
        vec![("Host".to_string(), "example.com".to_string())],
        None,
        0,
        None,
    );
    traffic.complete(1, 200, None, vec![], None, 0, false);

    traffic.begin_request(
        2,
        "POST".to_string(),
        "http://example.com/2".to_string(),
        "example.com".to_string(),
        vec![],
        Some("data".to_string()),
        4,
        None,
    );
    traffic.complete(2, 201, None, vec![], None, 0, false);

    traffic.begin_request(
        3,
        "GET".to_string(),
        "http://example.com/3".to_string(),
        "example.com".to_string(),
        vec![],
        None,
        0,
        None,
    );
    traffic.complete(3, 404, None, vec![], None, 0, false);

    // 批量查询 [2, 1, 3] - 应该按请求顺序返回
    let batch = traffic.get_batch(&[2, 1, 3]);

    assert_eq!(batch.len(), 3);
    assert_eq!(batch[0].as_ref().unwrap().summary.method, "POST");
    assert_eq!(batch[1].as_ref().unwrap().summary.method, "GET");
    assert_eq!(batch[2].as_ref().unwrap().summary.status, Some(404));
}

#[test]
fn test_get_batch_handles_missing_ids() {
    let traffic = SharedTraffic::new();

    traffic.begin_request(
        10,
        "GET".to_string(),
        "http://example.com".to_string(),
        "example.com".to_string(),
        vec![],
        None,
        0,
        None,
    );
    traffic.complete(10, 200, None, vec![], None, 0, false);

    // 查询 [10, 999, 10] - 999 不存在
    let batch = traffic.get_batch(&[10, 999, 10]);

    assert_eq!(batch.len(), 3);
    assert!(batch[0].is_some());
    assert!(batch[1].is_none()); // 不存在的 ID
    assert!(batch[2].is_some());
}

#[test]
fn test_summary_ring_keeps_latest_500_entries() {
    let traffic = SharedTraffic::new();

    for id in 1..=501 {
        traffic.begin_request(
            id,
            "GET".to_string(),
            format!("http://example.com/{id}"),
            "example.com".to_string(),
            vec![],
            None,
            0,
            None,
        );
        traffic.complete(id, 200, None, vec![], None, 0, false);
    }

    let list = traffic.list(1000, 0);

    assert_eq!(list.len(), 500);
    assert_eq!(list.first().unwrap().id, 2);
    assert_eq!(list.last().unwrap().id, 501);
    assert!(traffic.get(1).is_none());
    assert!(traffic.get(501).is_some());
}

#[test]
fn test_visible_summaries_keep_matching_details_after_cache_rollover() {
    let traffic = SharedTraffic::new();

    for id in 1..=700 {
        traffic.begin_request(
            id,
            "GET".to_string(),
            format!("http://example.com/{id}"),
            "example.com".to_string(),
            vec![],
            None,
            0,
            None,
        );
        traffic.complete(id, 200, None, vec![], None, 0, false);
    }

    let missing_ids: Vec<u64> = traffic
        .list(1000, 0)
        .into_iter()
        .map(|summary| summary.id)
        .filter(|id| traffic.get(*id).is_none())
        .collect();

    assert!(
        missing_ids.is_empty(),
        "visible summaries without details: {missing_ids:?}"
    );
}

#[test]
fn test_delete_removes_summaries_and_details() {
    let traffic = SharedTraffic::new();

    for id in 1..=3 {
        traffic.begin_request(
            id,
            "GET".to_string(),
            format!("http://example.com/{id}"),
            "example.com".to_string(),
            vec![],
            None,
            0,
            None,
        );
        traffic.complete(id, 200, None, vec![], None, 0, false);
    }

    traffic.delete(&[2]);

    let ids: Vec<u64> = traffic
        .list(10, 0)
        .into_iter()
        .map(|item| item.id)
        .collect();
    assert_eq!(ids, vec![1, 3]);
    assert!(traffic.get(1).is_some());
    assert!(traffic.get(2).is_none());
    assert!(traffic.get(3).is_some());
}

#[test]
fn test_form_urlencoded_body_is_captured() {
    assert!(should_capture_content_type(Some(
        "application/x-www-form-urlencoded",
    )));
    assert!(should_capture_content_type(Some(
        "application/x-www-form-urlencoded; charset=UTF-8",
    )));
}

#[test]
fn test_sse_body_is_not_captured() {
    assert!(!should_capture_content_type(Some("text/event-stream")));
}
