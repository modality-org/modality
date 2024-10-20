use modality_network_datastore::NetworkDatastore;

#[tokio::test]
async fn test_network_datastore() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("test_db");
    let datastore = NetworkDatastore::new(&path).unwrap();

    // Test set and get
    datastore.set_data_by_key("/test/key1", b"value1").await.unwrap();
    let value = datastore.get_data_by_key("/test/key1").await.unwrap().unwrap();
    assert_eq!(value, b"value1");

    // Test get_string
    let string_value = datastore.get_string("/test/key1").await.unwrap().unwrap();
    assert_eq!(string_value, "value1");

    // Test JSON
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct TestStruct {
        field: String,
    }
    let test_struct = TestStruct { field: "test".to_string() };
    datastore.put("/test/json", &serde_json::to_vec(&test_struct).unwrap()).await.unwrap();
    let retrieved: TestStruct = datastore.get_json("/test/json").await.unwrap().unwrap();
    assert_eq!(retrieved, test_struct);

    // Test find_max_string_key
    datastore.set_data_by_key("/pages/1", b"").await.unwrap();
    datastore.set_data_by_key("/pages/2", b"").await.unwrap();
    datastore.set_data_by_key("/pages/3", b"").await.unwrap();
    let max_key = datastore.find_max_string_key("/pages").await.unwrap().unwrap();
    assert_eq!(max_key, "3");

    // Test find_max_int_key
    datastore.set_data_by_key("/pages/10", b"").await.unwrap();
    datastore.set_data_by_key("/pages/20", b"").await.unwrap();
    let max_int_key = datastore.find_max_int_key("/pages").await.unwrap().unwrap();
    assert_eq!(max_int_key, 20);

    // Test current round operations
    datastore.set_current_round(5).await.unwrap();
    let current_round = datastore.get_current_round().await.unwrap();
    assert_eq!(current_round, 5);

    let new_round = datastore.bump_current_round().await.unwrap();
    assert_eq!(new_round, 6);


    // Test iteration within prefix
    datastore.set_data_by_key("/consensus/round_messages/1/type/type1/scribe/scribe1", b"").await.unwrap();
    datastore.set_data_by_key("/consensus/round_messages/1/type/type1/scribe/scribe2", b"").await.unwrap();
    datastore.set_data_by_key("/consensus/round_messages/1/type/type1/scribe/scribe3", b"").await.unwrap();
    datastore.set_data_by_key("/consensus/round_messages/1/type/type1a/scribe/scribe1", b"").await.unwrap();
    datastore.set_data_by_key("/consensus/round_messages/1/type/type2/scribe/scribe1", b"").await.unwrap();
    datastore.set_data_by_key("/consensus/round_messages/1/type/type10/scribe/scribe1", b"").await.unwrap();
    let iterator = datastore.iterator(&"/consensus/round_messages/1/type/type1");
    assert_eq!(iterator.count(), 3);
}