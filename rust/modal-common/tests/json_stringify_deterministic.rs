use serde_json::json;
use modal_common::json_stringify_deterministic::stringify_deterministic;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_object() {
        let obj = json!({
            "c": 6,
            "b": [4, 5],
            "a": 3,
            "z": null
        });
        assert_eq!(
            stringify_deterministic(&obj, None),
            r#"{"a":3,"b":[4,5],"c":6,"z":null}"#
        );
    }

    // NB: UNDEFINED IS NOT SUPPORTED
    // mod test_undefined {
    //     use super::*;

    //     #[test]
    //     fn in_object() {
    //         let obj = json!({
    //             "a": 3,
    //             // "z": null  // Rust JSON doesn't have undefined, so we use null
    //         });
    //         assert_eq!(
    //             stringify_deterministic(&obj, None),
    //             r#"{"a":3}"#
    //         );
    //     }

    //     #[test]
    //     fn in_array() {
    //         let obj = json!([4, null, 6]);  // Rust JSON doesn't have undefined, so we use null
    //         assert_eq!(
    //             stringify_deterministic(&obj, None),
    //             r#"[4,null,6]"#
    //         );
    //     }
    // }

    mod test_empty_string {
        use super::*;

        #[test]
        fn in_object() {
            let obj = json!({
                "a": 3,
                "z": ""
            });
            assert_eq!(
                stringify_deterministic(&obj, None),
                r#"{"a":3,"z":""}"#
            );
        }

        #[test]
        fn in_array() {
            let obj = json!([4, "", 6]);
            assert_eq!(
                stringify_deterministic(&obj, None),
                r#"[4,"",6]"#
            );
        }
    }

    mod test_regex {
        use super::*;

        #[test]
        fn in_object() {
            let obj = json!({
                "a": 3,
                "z": "/foobar/"  // We use a string representation of regex
            });
            assert_eq!(
                stringify_deterministic(&obj, None),
                r#"{"a":3,"z":"/foobar/"}"#
            );
        }

        #[test]
        fn in_array() {
            let obj = json!([4, null, "/foobar/"]);  // We use null for undefined and a string for regex
            assert_eq!(
                stringify_deterministic(&obj, None),
                r#"[4,null,"/foobar/"]"#
            );
        }
    }
}