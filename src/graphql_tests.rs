use std::{fs, sync::LazyLock};

use anyhow::{anyhow, Result};
use diff_logger::DiffLogger;
use reqwest::Method;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use tracing::{error, info};

use crate::{
    query_info::Schema,
    request::{MOCK_API_CLIENT, REFERENCE_GRAPHQL_CLIENT, TESTED_GRAPHQL_CLIENT},
    type_info::Root,
};

use super::ROOT_DIR;

const NUMBER_OF_TESTS: usize = 5;

static TESTS: LazyLock<Result<Vec<String>>> = LazyLock::new(|| {
    let tests_path = format!("{ROOT_DIR}/tests");
    let mut tests = Vec::new();

    for entry in fs::read_dir(tests_path)? {
        let path = entry?.path();

        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext == "graphql" {
                tests.push(fs::read_to_string(&path)?);
            }
        }
    }

    Ok(tests)
});

pub async fn run_graphql_tests() -> Result<()> {
    info!("Run graphql assert tests");

    let tests = TESTS
        .as_ref()
        .map_err(|e| anyhow!("Failed to resolve tests due to error: {e}"))?;

    for i in 1..NUMBER_OF_TESTS {
        info!("Test iteration: {i}");

        MOCK_API_CLIENT.request(Method::POST, "reset").await?;

        for test in tests {
            let actual = TESTED_GRAPHQL_CLIENT.request(&test).await?;

            let expected = REFERENCE_GRAPHQL_CLIENT.request(&test).await?;

            let differ = DiffLogger::new();

            let difference = differ.diff(&expected, &actual);

            if !difference.is_empty() {
                error!(
                    "Actual response is not equal to expected
    Note: left is expected response -> right is actual response"
                );
                println!("{}", difference);

                return Err(anyhow!("Actual response is not equal to expected"));
            }
        }
    }

    info!("Execution of graphql tests finished");

    Ok(())
}

fn query_builder(type_name: &str) -> String {
    format!(
        r#"
        {{
          __type(name: "{}") {{
            name
            kind
            fields {{
              name
              args {{
                name
              }}
            }}
          }}
        }}
        "#,
        type_name
    )
}

fn compare<T: DeserializeOwned + Serialize + Clone>(
    actual: Value,
    expected: Value,
    error_message: &str,
) -> Result<()> {
    // in order to have sorting.
    let actual: T = serde_json::from_value(actual)?;
    let expected: T = serde_json::from_value(expected)?;

    // with value we can compare with diff logger
    let actual_value = serde_json::to_value(actual)?;
    let expected_value = serde_json::to_value(expected)?;

    let differ = DiffLogger::new();

    let difference = differ.diff(&actual_value, &expected_value);
    if !difference.is_empty() {
        error!(error_message);
        println!("{}", difference);
        return Err(anyhow!(error_message.to_owned()));
    }

    Ok(())
}

pub async fn run_introspection_query() -> Result<()> {
    info!("Run graphql introspection tests");
    let query_info = include_str!("./query_info.graphql");

    // check the root query is same or not.
    let actual_value = TESTED_GRAPHQL_CLIENT.request(&query_info).await?;
    let expected_value = REFERENCE_GRAPHQL_CLIENT.request(&query_info).await?;
    let actual: Schema = serde_json::from_value(actual_value.clone())?;
    let expected: Schema = serde_json::from_value(expected_value.clone())?;

    let _ = compare::<Schema>(
        actual_value,
        expected_value,
        "Query Operation type mismatch",
    )?;

    for (actual, expected) in actual
        .data
        .schema
        .query_type
        .fields
        .iter()
        .zip(expected.data.schema.query_type.fields.iter())
    {
        let actual_op_type = actual.field_type.get_name();
        let expected_op_type = expected.field_type.get_name();

        if actual_op_type.is_none() != expected_op_type.is_none() {
            error!("Output type mismatch for field: {:?}", actual.name);
            return Err(anyhow!("Output type mismatch for field: {:?}", actual.name));
        }

        let actual_op_type = actual_op_type.unwrap();
        let expected_op_type = expected_op_type.unwrap();

        let actual_op_type_query = query_builder(&actual_op_type);
        let expected_op_type_query = query_builder(&expected_op_type);

        let actual = TESTED_GRAPHQL_CLIENT.request(&actual_op_type_query).await?;
        let expected = REFERENCE_GRAPHQL_CLIENT
            .request(&expected_op_type_query)
            .await?;

        let _ = compare::<Root>(actual, expected, "Type Defination mismatch")?;
    }

    info!("Execution of graphql schema validation finished");

    Ok(())
}
