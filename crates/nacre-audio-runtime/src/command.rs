//! Bounded SPSC command queue used between the control plane and the callback.

use nacre_application::{CommandError, ParameterCommand};
use rtrb::{Consumer, Producer, PushError, RingBuffer};

use crate::COMMAND_QUEUE_CAPACITY;

pub(crate) struct CommandSender {
    producer: Producer<ParameterCommand>,
}

pub(crate) fn command_queue() -> (CommandSender, Consumer<ParameterCommand>) {
    let (producer, consumer) = RingBuffer::new(COMMAND_QUEUE_CAPACITY);
    (CommandSender { producer }, consumer)
}

impl CommandSender {
    /// Pushes without ever blocking; a full queue is reported to the caller.
    pub(crate) fn submit(&mut self, command: ParameterCommand) -> Result<(), CommandError> {
        match self.producer.push(command) {
            Ok(()) => Ok(()),
            Err(PushError::Full(_)) => Err(CommandError::Full),
        }
    }
}

#[cfg(test)]
mod tests {
    use nacre_application::{CommandError, ParameterCommand};

    use super::command_queue;
    use crate::COMMAND_QUEUE_CAPACITY;

    fn command(index: usize) -> ParameterCommand {
        ParameterCommand::SetFrequency {
            frequency_hz: 100.0 + index as f32,
        }
    }

    #[test]
    fn queue_accepts_capacity_and_reports_full_without_blocking() {
        let (mut sender, mut receiver) = command_queue();

        for index in 0..COMMAND_QUEUE_CAPACITY {
            assert_eq!(sender.submit(command(index)), Ok(()));
        }
        assert_eq!(
            sender.submit(command(COMMAND_QUEUE_CAPACITY)),
            Err(CommandError::Full)
        );

        for index in 0..COMMAND_QUEUE_CAPACITY {
            assert_eq!(receiver.pop(), Ok(command(index)));
        }
        assert!(receiver.pop().is_err());
    }

    #[test]
    fn draining_frees_capacity() {
        let (mut sender, mut receiver) = command_queue();

        for index in 0..COMMAND_QUEUE_CAPACITY {
            sender.submit(command(index)).expect("queue has capacity");
        }
        receiver.pop().expect("one command was queued");
        assert_eq!(sender.submit(command(0)), Ok(()));
    }
}
