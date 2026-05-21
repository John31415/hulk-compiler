use serde::Serialize;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Copy, Serialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn from_range(range: Range<usize>) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }

    pub fn union(&self, right: &Self) -> Self {
        Self {
            start: self.start,
            end: right.end,
        }
    }
}
