use modality_datastore::DatastoreManager;

#[tokio::test]
async fn keys_values_and_rounds() {
    let temp_dir = tempfile::tempdir().unwrap();
    let datastore = DatastoreManager::open(&temp_dir.path().join("test_db")).unwrap();

    datastore.set_data_by_key("/test/key1", b"value1").await.unwrap();
    let value = datastore.get_data_by_key("/test/key1").await.unwrap().unwrap();
    assert_eq!(value, b"value1");
    assert_eq!(datastore.get_string("/test/key1").await.unwrap().unwrap(), "value1");

    datastore.put("/test/key2", b"value2").await.unwrap();
    assert_eq!(datastore.get_string("/test/key2").await.unwrap().unwrap(), "value2");
    datastore.delete("/test/key2").await.unwrap();
    assert!(datastore.get_data_by_key("/test/key2").await.unwrap().is_none());

    datastore.set_current_round(5).await.unwrap();
    assert_eq!(datastore.get_current_round().await.unwrap(), 5);
    assert_eq!(datastore.bump_current_round().await.unwrap(), 6);
}
