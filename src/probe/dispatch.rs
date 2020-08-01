use std::collections::hash_map::RandomState;
use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender};

use crate::probe::{Error, Probe, ProcessMetric};
use crate::process::PID;
use crate::values::{Bitrate, Percent};

#[derive(PartialEq, Debug)]
pub enum Metrics {
    Percents(Vec<ProcessMetric<Percent>>),
    Bitrates(Vec<ProcessMetric<Bitrate>>),
}

