use irgen::register_map::Lifetime;
use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::{BTreeSet, BinaryHeap},
    rc::Rc,
    vec::Vec,
};

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Interval {
    lifetime: Lifetime,
    register: Option<usize>,
    location: Option<usize>,
}

impl Interval {
    pub fn set_register(&mut self, r: usize) {
        self.register = Some(r);
        self.location = None;
    }

    pub fn set_location(&mut self, l: usize) {
        self.register = None;
        self.location = Some(l);
    }
}

impl PartialOrd for Interval {
    fn partial_cmp(&self, other: &Interval) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Interval {
    fn cmp(&self, other: &Interval) -> Ordering {
        let mut ord = self.lifetime.end_point().cmp(&other.lifetime.end_point());
        if ord == Ordering::Equal {
            ord = self
                .lifetime
                .start_point()
                .cmp(&other.lifetime.start_point());
        }
        ord
    }
}

pub struct LinearScan {
    target: usize,
    ml: MemoryLocations,
    intervals: Vec<Rc<RefCell<Interval>>>,
    active: BTreeSet<Rc<RefCell<Interval>>>,
    free_regs: BinaryHeap<usize>,
}

impl LinearScan {
    pub fn get_reg_allocation(target: usize, lifetimes: &Vec<Lifetime>) {
        LinearScan {
            target,
            ml: MemoryLocations::new(),
            intervals: (*lifetimes)
                .clone()
                .iter()
                .map(|lifetime| {
                    Rc::new(RefCell::new(Interval {
                        lifetime: lifetime.clone(),
                        register: None,
                        location: None,
                    }))
                })
                .collect(),
            active: BTreeSet::new(),
            free_regs: (0..target).into_iter().collect(),
        }
        .allocate();
    }

    fn allocate(&mut self) {
        for i in 0..self.intervals.len() {
            let start_point = self.intervals[i].borrow().lifetime.start_point();
            self.expire_old_intervals(start_point);
            if self.active.len() == self.target {
                let interval = Rc::clone(&self.intervals[i]);
                self.spill_at_interval(interval);
            } else {
                self.intervals[i]
                    .borrow_mut()
                    .set_register(self.free_regs.pop().unwrap());
                self.active.insert(Rc::clone(&self.intervals[i]));
            }
        }
        for i in &self.intervals {
            println!("{:?}", i);
        }
    }

    fn expire_old_intervals(&mut self, sp: usize) {
        let mut to_delete = BTreeSet::new();
        // which registers become free after intervals expire
        for lj in self.active.iter() {
            if lj.borrow().lifetime.end_point() >= sp {
                break;
            }
            to_delete.insert(Rc::clone(lj));
            self.free_regs.push(lj.borrow().register.unwrap());
        }
        self.active = self
            .active
            .difference(&to_delete)
            .map(|j| Rc::clone(j))
            .collect();
    }

    fn spill_at_interval(&mut self, interval: Rc<RefCell<Interval>>) {
        let spill = Rc::clone(self.active.iter().last().unwrap());
        if spill.borrow().lifetime.end_point() > interval.borrow().lifetime.end_point() {
            interval.borrow_mut().register = spill.borrow().register;
            spill.borrow_mut().set_location(self.ml.get_location());
            self.active.remove(&spill);
            self.active.insert(Rc::clone(&interval));
        } else {
            interval.borrow_mut().set_location(self.ml.get_location());
        }
    }
}

struct MemoryLocations {
    n: usize,
}

impl MemoryLocations {
    pub fn new() -> MemoryLocations {
        MemoryLocations { n: 0 }
    }

    pub fn get_location(&mut self) -> usize {
        let to_ret = self.n;
        self.n += 1;
        to_ret
    }
}
