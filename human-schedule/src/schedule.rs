use crate::Schedule;
use time::PrimitiveDateTime;

impl Schedule {
    pub fn next_schedule(&self, after: PrimitiveDateTime) -> PrimitiveDateTime {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
