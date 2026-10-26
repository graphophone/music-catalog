use crate::config;

pub struct AudioProcessingQueue {
    channel: lapin::Channel,
    queue: lapin::types::ShortString,
}

impl AudioProcessingQueue {
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

        Ok(AudioProcessingQueue { channel, queue })
    }

    pub async fn send_audio_processing_request(
        &self,
        music_id: i64,
        audio_id: &str,
    ) -> anyhow::Result<()> {
        let req = serde_json::json!({
            "music_id": music_id,
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
