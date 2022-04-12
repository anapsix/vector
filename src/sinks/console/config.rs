use codecs::{encoding::Framer, NewlineDelimitedEncoder};
use futures::{future, FutureExt};
use serde::{Deserialize, Serialize};
use tokio::io;

use crate::{
    codecs::Encoder,
    config::{AcknowledgementsConfig, GenerateConfig, Input, SinkConfig, SinkContext},
    sinks::{
        console::sink::WriterSink,
        util::encoding::{
            EncodingConfig, EncodingConfigWithFramingAdapter, StandardEncodings,
            StandardEncodingsWithFramingMigrator,
        },
        Healthcheck, VectorSink,
    },
};

#[derive(Debug, Derivative, Deserialize, Serialize)]
#[derivative(Default)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    #[derivative(Default)]
    Stdout,
    Stderr,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ConsoleSinkConfig {
    #[serde(default)]
    pub target: Target,
    #[serde(flatten)]
    pub encoding: EncodingConfigWithFramingAdapter<
        EncodingConfig<StandardEncodings>,
        StandardEncodingsWithFramingMigrator,
    >,
}

impl GenerateConfig for ConsoleSinkConfig {
    fn generate_config() -> toml::Value {
        toml::Value::try_from(Self {
            target: Target::Stdout,
            encoding: EncodingConfig::from(StandardEncodings::Json).into(),
        })
        .unwrap()
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "console")]
impl SinkConfig for ConsoleSinkConfig {
    async fn build(&self, cx: SinkContext) -> crate::Result<(VectorSink, Healthcheck)> {
        let transformer = self.encoding.transformer();
        let (framer, serializer) = self.encoding.clone().encoding();
        let framer = match (framer, &serializer) {
            (Some(framer), _) => framer,
            (
                None,
                codecs::encoding::Serializer::Json(_) | codecs::encoding::Serializer::RawMessage(_),
            ) => NewlineDelimitedEncoder::new().into(),
        };
        let encoder = Encoder::<Framer>::new(framer, serializer);

        let sink: VectorSink = match self.target {
            Target::Stdout => VectorSink::from_event_streamsink(WriterSink {
                acker: cx.acker(),
                output: io::stdout(),
                transformer,
                encoder,
            }),
            Target::Stderr => VectorSink::from_event_streamsink(WriterSink {
                acker: cx.acker(),
                output: io::stderr(),
                transformer,
                encoder,
            }),
        };

        Ok((sink, future::ok(()).boxed()))
    }

    fn input(&self) -> Input {
        Input::all()
    }

    fn sink_type(&self) -> &'static str {
        "console"
    }

    fn acknowledgements(&self) -> Option<&AcknowledgementsConfig> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::test_util::test_generate_config::<ConsoleSinkConfig>();
    }
}
