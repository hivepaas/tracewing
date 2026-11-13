//! Where the user has been, to go back and forward through as a browser does:
//! with the mouse's back and forward buttons, or Alt+Left and Alt+Right.

/// How many places are kept: the oldest go first.
const KEPT: usize = 100;

/// Places in the order they were gone to, and the one the user is at.
#[derive(Debug)]
pub struct History<T> {
    places: Vec<T>,
    at: usize,
}

impl<T: Clone + PartialEq> History<T> {
    pub fn new(here: T) -> Self {
        History {
            places: vec![here],
            at: 0,
        }
    }

    /// The user went to `here` themselves: what was ahead is forgotten.
    pub fn went(&mut self, here: T) {
        if self.places[self.at] == here {
            return;
        }
        self.places.truncate(self.at + 1);
        self.places.push(here);
        if self.places.len() > KEPT {
            self.places.remove(0);
        }
        self.at = self.places.len() - 1;
    }

    /// The nearest place back, or forward, that is not where the user is and
    /// that `can` go to: what is gone has nothing to show, and is passed.
    pub fn next(&self, back: bool, can: impl Fn(&T) -> bool) -> Option<(usize, T)> {
        let here = &self.places[self.at];
        let fits = |&i: &usize| self.places[i] != *here && can(&self.places[i]);
        let found = if back {
            (0..self.at).rev().find(fits)
        } else {
            (self.at + 1..self.places.len()).find(fits)
        };
        found.map(|i| (i, self.places[i].clone()))
    }

    /// The user is at the place [`History::next`] found, as it turned out to be:
    /// it may not show all it did.
    pub fn moved(&mut self, to: usize, here: T) {
        self.at = to.min(self.places.len() - 1);
        self.places[self.at] = here;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn back_and_forward_go_through_the_places_in_order() {
        let mut h = History::new(1);
        h.went(2);
        h.went(3);
        let (i, back) = h.next(true, |_| true).expect("back");
        assert_eq!(back, 2);
        h.moved(i, back);
        let (i, back) = h.next(true, |_| true).expect("back");
        assert_eq!(back, 1);
        h.moved(i, back);
        assert_eq!(h.next(true, |_| true), None);
        let (i, ahead) = h.next(false, |_| true).expect("forward");
        assert_eq!(ahead, 2);
        h.moved(i, ahead);
        // Going somewhere forgets what was ahead.
        h.went(4);
        assert_eq!(h.next(false, |_| true), None);
        assert_eq!(h.next(true, |_| true).map(|(_, p)| p), Some(2));
    }

    #[test]
    fn staying_put_is_not_a_place() {
        let mut h = History::new(1);
        h.went(1);
        assert_eq!(h.next(true, |_| true), None);
    }

    #[test]
    fn places_that_are_gone_or_where_one_is_are_passed() {
        let mut h = History::new(1);
        h.went(2);
        h.went(1);
        h.went(3);
        assert_eq!(h.next(true, |p| *p != 1).map(|(_, p)| p), Some(2));
        // Gone back to a place that showed less: it is kept as it was.
        let (i, _) = h.next(true, |_| true).expect("back");
        h.moved(i, 3);
        assert_eq!(h.next(true, |_| true).map(|(_, p)| p), Some(2));
        assert_eq!(h.next(false, |_| true), None);
    }

    #[test]
    fn the_oldest_go_first() {
        let mut h = History::new(0);
        for i in 1..=KEPT + 10 {
            h.went(i);
        }
        let mut oldest = KEPT + 10;
        while let Some((i, p)) = h.next(true, |_| true) {
            h.moved(i, p);
            oldest = p;
        }
        assert_eq!(oldest, 11);
    }
}
