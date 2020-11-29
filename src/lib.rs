use std::collections::HashMap;

// LatOpt is the bare minimum information to build a lattice out of
pub trait LatOpt<T> {
    // new creates a new LatOpt out of the base data type
    fn new(t: T) -> Self;
    // returns the UUID of the LatOpt
    fn uuid(&self) -> String;
    // returns the list of UUIDs that the LatOpt depends on
    fn depends_list(&self) -> Vec<String>;
    // returns if the base data type is completed
    fn is_completed(&self) -> bool;
}

pub trait ReadLatNode<T>: LatOpt<T> {
    fn from_lat_opt<A>(l: A) -> Self
    where
        A: LatOpt<T>;
    // Once depends_on and fulfilled_by are satisfied, we get
    // depends_list for free, not that it's particularly useful.
    fn depends_list(&self) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        let mut k1: Vec<String> = self.depends_on().keys().cloned().collect();
        v.append(&mut k1);

        let mut k2: Vec<String> = self.fulfilled_by().keys().cloned().collect();
        v.append(&mut k2);

        v
    }
    // returns a hashmap where keys are the LatNode who this LatNode depends on being
    // complete before becoming active and a bool indicating whether that LatNode
    // is active or not
    fn depends_on(&self) -> &HashMap<String, ()>;
    // returns a hashmap where keys are the LatNodes who require this LatNode
    fn required_by(&self) -> &HashMap<String, ()>;
    // returns a hashmap where keys are the completed LatNodes this depended on.
    fn fulfilled_by(&self) -> &HashMap<String, ()>;
    // returns whether this LatNode should be active.
    fn active(&self) -> bool {
        if self.is_completed() {
            return false;
        };

        self.depends_on().is_empty()
    }
}

pub trait WriteLatNode<T>: ReadLatNode<T> {
    fn from_read_lat<A>(l: A) -> Self
    where
        A: ReadLatNode<T>;
    // returns a mutable reference to a hashmap where keys are the
    // LatNode who this LatNode depends on being
    // complete before becoming active and a bool indicating whether
    // that LatNode is active or not
    fn get_depends_on(&mut self) -> &mut HashMap<String, ()>;
    // returns a mutable reference to a hashmap where keys are the
    // LatNodes who require this LatNode
    fn get_required_by(&mut self) -> &mut HashMap<String, ()>;
    // returns a hashmap where keys are the completed LatNodes this
    // depended on
    fn get_fulfilled_by(&mut self) -> &mut HashMap<String, ()>;
    // returns a hashmap where keys are the completed LatNodes this
    // depended on.

    // add_depends_on takes a LatNode's UUID and adds it to the relations
    fn add_depends_on(&mut self, key: String) {
        self.get_depends_on().insert(key, ());
    }

    // add_required_by takes a LatNode's UUID and adds it to the relations
    fn add_required_by(&mut self, key: String) {
        self.get_required_by().insert(key, ());
    }

    fn depend_fulfilled(&mut self, key: String) -> Result<(), ()> {
        match self.get_depends_on().remove(&key) {
            None => Err(()),
            Some(_) => {
                self.get_fulfilled_by().insert(key, ());
                Ok(())
            }
        }
    }

    fn update(&mut self, t: T) -> Result<(), ()>;
}

pub trait LatMachine<T, U>
where
    T: WriteLatNode<U>,
{
    // default impl?
    fn new() -> Self;

    // If embedding as a LatNode, then we get this for free.
    fn is_completed(&self) -> bool {
        self.read_pending().is_empty()
    }

    fn read_pending(&self) -> &HashMap<String, T>;
    fn read_fulfilled(&self) -> &HashMap<String, T>;

    fn get_pending(&mut self) -> &mut HashMap<String, T>;
    fn get_fulfilled(&mut self) -> &mut HashMap<String, T>;

    fn append_pending(&mut self, t: T) {
        self.get_pending().insert(t.uuid(), t);
    }

    fn append_fulfilled(&mut self, t: T) {
        self.get_fulfilled().insert(t.uuid(), t);
    }

    fn append(&mut self, t: T) {
        if t.is_completed() {
            self.append_fulfilled(t);
        } else {
            self.append_pending(t);
        };
    }

    // Always public below here:
    fn fulfill(&mut self, key: String) -> Result<(), ()> {
        let pending = self.get_pending();
        match pending.remove(&key) {
            None => Err(()),
            Some(mut target) => {
                let r_map = target.get_required_by();
                for k in r_map {
                    match pending.get_mut(k.0) {
                        // We assume it is in fulfilled.
                        None => {}
                        Some(x) => match x.depend_fulfilled(key.clone()) {
                            Ok(()) => {}
                            Err(()) => return Err(()),
                        },
                    };
                }

                self.get_fulfilled().insert(key, target);
                Ok(())
            }
        }
    }

    fn update_value(&mut self, key: String, update: U) -> Result<(), ()> {
        let m = self.get_pending();

        // TODO: REPORT ERRS, DYING HERE WOULD BE AWFUL.
        let x = m.get_mut(&key);
        match x {
            None => Err(()),
            Some(t) => match t.update(update) {
                Err(()) => Err(()),
                Ok(()) => {
                    if t.is_completed() {
                        match self.fulfill(key) {
                            Err(()) => return Err(()),
                            Ok(()) => return Ok(()),
                        };
                    };
                    Ok(())
                }
            },
        }
    }

    // boolean indicates whether this relationship blocks the value at is_required_by
    fn update_required_by(&mut self, target: String, is_required_by: String) -> Result<bool, ()> {
        // Is the required elm in pending?
        match self.get_pending().get_mut(&target) {
            Some(t) => {
                t.add_required_by(is_required_by);
                // It IS blocking
                Ok(true)
            }

            // The required elm must be fulfilled:
            None => match self.get_fulfilled().get_mut(&target) {
                // if not err.
                None => Err(()),
                Some(t) => {
                    t.add_required_by(is_required_by);
                    // It is NOT blocking.
                    Ok(false)
                }
            },
        }
    }

    fn update_depends_on(&mut self, target: String, depends_on: String) -> Result<(), ()> {
        // Is the required elm in pending?
        match self.get_pending().get_mut(&target) {
            Some(t) => {
                t.add_depends_on(depends_on);
                Ok(())
            }

            // The required elm must be fulfilled:
            None => Err(())
        }
    }
}

pub struct BasicLattice<T> {
    pending: HashMap<String, T>,
    fulfilled: HashMap<String, T>,
}

impl<T, U> LatMachine<T, U> for BasicLattice<T>
where
    T: WriteLatNode<U>,
{
    fn new() -> Self {
        BasicLattice {
            pending: HashMap::new(),
            fulfilled: HashMap::new(),
        }
    }

    fn read_pending(&self) -> &HashMap<String, T> {
        &self.pending
    }

    fn read_fulfilled(&self) -> &HashMap<String, T> {
        &self.fulfilled
    }

    fn get_pending(&mut self) -> &mut HashMap<String, T> {
        &mut self.pending
    }

    fn get_fulfilled(&mut self) -> &mut HashMap<String, T> {
        &mut self.fulfilled
    }
}
