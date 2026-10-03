use anyhow::Result;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{Client, config::Credentials, primitives::ByteStream};
use uuid::Uuid;
use crate::config;

pub struct AssetStorage {
    bucket: String,
    client: Client,
}

impl AssetStorage {
    pub async fn build(conf: &config::RustfsConfig) -> Result<Self> {
        let credentials = Credentials::new(
            &conf.access_key,
            &conf.secret_key,
            None,
            None,
            "rustfs",
        );

        let shared_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(conf.region.clone()))
            .credentials_provider(credentials)
            .endpoint_url(&conf.endpoint_url)
            .load()
            .await;

        let s3_config = aws_sdk_s3::config::Builder::from(&shared_config)
            .force_path_style(true)
            .build();

        let rustfs_client = Client::from_conf(s3_config);
        Self::ensure_bucket(&rustfs_client, &conf.assets_bucket).await;

        Ok(AssetStorage {
            client: rustfs_client,
            bucket: conf.assets_bucket.clone(),
        })
    }

    pub async fn upload_asset(&self, image: &[u8], mime_type: &str) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let id = format!("{}.{}", id, mime_type);
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&id)
            .body(ByteStream::from(image.to_vec()))
            .send()
            .await?;
        Ok(id)
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

    async fn ensure_bucket(rustfs_client: &Client, bucket_name: &str) {
        let buckets = rustfs_client
            .list_buckets()
            .send()
            .await
            .expect("Failed to retrieve buckets");
        buckets.buckets().iter()
            .find(|&b| b.name == Some(bucket_name.to_string()))
            .expect(&format!("Bucket {bucket_name} does not exist"));
    }
}