use std::sync::mpsc as std_channel;
#[cfg(feature = "tokio-channels")]
use tokio::sync::mpsc as tokio_channel;

#[derive(Clone)]
pub(crate) enum ChannelHelper<T> {
    StdChannel(std_channel::Sender<T>),
    #[cfg(feature = "tokio-channels")]
    TokioChannel(tokio_channel::UnboundedSender<T>),
}

impl ChannelHelper<(usize, String)> {
    pub(crate) fn send(&self, message: (usize, String)) -> bool {
        let result = match self {
            ChannelHelper::StdChannel(sender) => sender.send(message).is_ok(),

            #[cfg(feature = "tokio-channels")]
            ChannelHelper::TokioChannel(unbounded_sender) => unbounded_sender.send(message).is_ok(),
        };

        result
    }
}
