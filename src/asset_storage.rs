use std::time::Duration;

use crate::config;
use anyhow::Result;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{
    Client, Config,
    config::Credentials,
    presigning::PresigningConfig,
    types::{self, ObjectIdentifier},
};
use uuid::Uuid;

pub struct AssetStorage {
    bucket: String,
    client: Client,
    offline_client: Client,
}

impl AssetStorage {
    pub async fn build(bucket: String, conf: &config::RustfsConfig) -> Result<Self> {
        let credentials =
            Credentials::new(&conf.access_key, &conf.secret_key, None, None, "rustfs");

        let shared_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(conf.region.clone()))
            .credentials_provider(credentials.clone())
            .endpoint_url(&conf.endpoint_url)
            .load()
            .await;

        let s3_config = aws_sdk_s3::config::Builder::from(&shared_config)
            .force_path_style(true)
            .build();

        let rustfs_client = Client::from_conf(s3_config);
        Self::ensure_bucket(&rustfs_client, &bucket).await;

        let offline_conf = Config::builder()
            .endpoint_url(&conf.deployed_url)
            .behavior_version_latest()
            .region(Region::new(conf.region.clone()))
            .credentials_provider(credentials)
            .force_path_style(true)
            .build();
        let offline_client = Client::from_conf(offline_conf);

        Ok(AssetStorage {
            client: rustfs_client,
            offline_client,
            bucket,
        })
    }

    pub fn prepare_asset_id(&self, id: &str) -> String {
        format!("{}/{id}", self.bucket.clone())
    }

    pub async fn get_upload_url(&self, mime_type: &str) -> Result<(String, String)> {
        let split = mime_type.split("/");
        let file_type = match split.skip(1).next() {
            Some(v) => v.to_string(),
            None => return Err(anyhow::Error::msg("failed to parse mime_type")),
        };
        let id = Uuid::new_v4().to_string();
        let id = format!("{}.{}", id, file_type);
        let put_request = self
            .offline_client
            .put_object()
            .bucket(&self.bucket)
            .key(&id)
            .content_type(mime_type)
            .presigned(
                PresigningConfig::builder()
                    .expires_in(Duration::from_mins(5))
                    .build()?,
            )
            .await?;
        Ok((put_request.uri().to_string(), id))
    }

    pub async fn remove_asset(&self, id: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(id)
            .send()
            .await?;
        Ok(())
    }

    pub async fn remove_objects_with_prefix(&self, prefix: String) -> Result<()> {
        let mut continuation_token = None;

        loop {
            let res = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(&prefix)
                .set_continuation_token(continuation_token.take())
                .send()
                .await?;

            let objects = res
                .contents()
                .iter()
                .filter_map(|obj| obj.key())
                .map(|key| ObjectIdentifier::builder().key(key).build())
                .collect::<Result<Vec<ObjectIdentifier>, _>>()?;

            if !objects.is_empty() {
                let delete = types::Delete::builder()
                    .set_objects(Some(objects))
                    .quiet(true)
                    .build()?;

                let out = self
                    .client
                    .delete_objects()
                    .bucket(&self.bucket)
                    .delete(delete)
                    .send()
                    .await?;

                if !out.errors().is_empty() {
                    anyhow::bail!("failed to delete some objects: {:?}", out.errors());
                }

                if res.is_truncated().unwrap_or(false) {
                    continuation_token = res.next_continuation_token().map(String::from);
                } else {
                    break;
                }
            }
        }

        Ok(())
    }

    async fn ensure_bucket(rustfs_client: &Client, bucket_name: &str) {
        let buckets = rustfs_client
            .list_buckets()
            .send()
            .await
            .expect("Failed to retrieve buckets");
        buckets
            .buckets()
            .iter()
            .find(|&b| b.name == Some(bucket_name.to_string()))
            .expect(&format!("Bucket {bucket_name} does not exist"));
    }
}
