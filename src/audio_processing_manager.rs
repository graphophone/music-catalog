use crate::config;

pub struct AudioProcessingManager {
    channel: lapin::Channel,
    queue: lapin::types::ShortString,
}

impl AudioProcessingManager {
    pub async fn build(conf: &config::RabbitmqConfig) -> anyhow::Result<Self> {
        let connection =
            lapin::Connection::connect(&conf.addr, lapin::ConnectionProperties::default()).await?;

        let queue = lapin::types::ShortString::from(conf.queue.as_ref());

        let channel = connection.create_channel().await?;
        channel
            .queue_declare(
                queue.clone(),
                lapin::options::QueueDeclareOptions::durable(),
                Default::default(),
            )
            .await?;

        Ok(AudioProcessingManager { channel, queue })
    }

    pub async fn send_audio_processing_request(
        &self,
        track_id: i64,
        audio_id: &str,
    ) -> anyhow::Result<()> {
        let req = serde_json::json!({
            "track_id": track_id,
            "audio_id": audio_id,
        });

        self.channel
            .basic_publish(
                "".into(),
                self.queue.clone(),
                lapin::options::BasicPublishOptions::default(),
                req.to_string().as_bytes(),
                lapin::BasicProperties::default(),
            )
            .await?
            .await?;

        Ok(())
    }
}
