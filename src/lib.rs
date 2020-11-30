use std::collections::HashMap;

// The base data and measure of "completed-ness" required to be
// useable inside of a lattice machine
pub trait NodeType {
    // returns the UUID of the NodeType to be used by the NodeOpt
    fn uuid(&self) -> String;
    // returns if the base data type is completed
    fn is_completed(&self) -> bool;
}

pub trait ReadNode<T>: NodeType {
    // new creates a new NodeOpt out of the base data type
    // and list of relations it depends on.
    fn new(base_data: T, depends_on: Vec<String>, required_by: Vec<String>) -> Self;

    // returns a hashmap where keys are the LatNode who this LatNode depends on being
    // complete before becoming active and a bool indicating whether that LatNode
    // is active or not
    fn depends_on(&self) -> &HashMap<String, ()>;
    // returns a hashmap where keys are the LatNodes who require this LatNode
    fn required_by(&self) -> &HashMap<String, ()>;
    // returns a hashmap where keys are the completed LatNodes this depended on.
    fn fulfilled_by(&self) -> &HashMap<String, ()>;
    // returns whether this LatNode should be active.
    fn is_active(&self) -> bool {
        !(!self.is_completed() && self.depends_on().is_empty())
    }

    fn is_pending(&self) -> bool {
        !self.depends_on().is_empty() || !self.is_completed()
    }
}

pub struct BasicNode<T>
where
    T: NodeType,
{
    base_data: T,
    depends_on: HashMap<String, ()>,
    required_by: HashMap<String, ()>,
    fulfilled_by: HashMap<String, ()>,
}

impl<T: NodeType> NodeType for BasicNode<T> {
    fn uuid(&self) -> String {
        self.base_data.uuid()
    }

    fn is_completed(&self) -> bool {
        self.base_data.is_completed()
    }
}

impl<T: NodeType> ReadNode<T> for BasicNode<T> {
    fn new(t: T, depends_on: Vec<String>, required_by: Vec<String>) -> Self {
        let mut s = Self {
            base_data: t,
            depends_on: HashMap::new(),
            fulfilled_by: HashMap::new(),
            required_by: HashMap::new(),
        };

        for v in depends_on {
            s.depends_on.insert(v, ());
        }

        for v in required_by {
            s.required_by.insert(v, ());
        }

        s
    }

    fn depends_on(&self) -> &HashMap<String, ()> {
        &self.depends_on
    }

    fn required_by(&self) -> &HashMap<String, ()> {
        &self.required_by
    }

    fn fulfilled_by(&self) -> &HashMap<String, ()> {
        &self.fulfilled_by
    }
}

pub trait WriteNode<T>: ReadNode<T> {
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

impl<T: NodeType> WriteNode<T> for BasicNode<T> {
    fn get_depends_on(&mut self) -> &mut HashMap<String, ()> {
        &mut self.depends_on
    }

    fn get_required_by(&mut self) -> &mut HashMap<String, ()> {
        &mut self.required_by
    }

    fn get_fulfilled_by(&mut self) -> &mut HashMap<String, ()> {
        &mut self.fulfilled_by
    }

    fn update(&mut self, t: T) -> Result<(), ()> {
        self.base_data = t;
        Ok(())
    }
}

pub trait LatMachine<T, U>
where
    T: WriteNode<U>,
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
        if t.is_pending() {
            self.append_pending(t);
        } else {
            self.append_fulfilled(t);
        };
    }

    // Always public below here:
    fn fulfill(&mut self, key: String) -> Result<(), ()> {
        let mut cascasde = Vec::<String>::new();
        let pending = self.get_pending();
        match pending.remove(&key) {
            None => Err(()),
            Some(mut target) => {
                let r_map = target.get_required_by();
                for (k, _) in r_map {
                    match pending.get_mut(k) {
                        None => return Err(()),
                        Some(x) => match x.depend_fulfilled(key.clone()) {
                            Ok(()) => {
                                if !x.is_pending() {
                                    cascasde.push(x.uuid());
                                }
                            }
                            Err(()) => return Err(()),
                        },
                    };
                }

                self.get_fulfilled().insert(key, target);

                for v in cascasde {
                    match self.fulfill(v) {
                        Err(()) => return Err(()),
                        Ok(()) => {}
                    };
                }

                Ok(())
            }
        }
    }

    // TODO: IMPLEMENT
    fn unfulfill(&mut self, key: String) -> Result<(), ()> {
        Ok(())
        // let mut cascasde = Vec::<String>::new();
        // let pending = self.get_pending();
        // match pending.remove(&key) {
        //     None => Err(()),
        //     Some(mut target) => {
        //         let r_map = target.get_required_by();
        //         for (k, _) in r_map {
        //             match pending.get_mut(k) {
        //                 None => return Err(()),
        //                 Some(x) => match x.depend_fulfilled(key.clone()) {
        //                     Ok(()) => {
        //                         if !x.is_pending() {
        //                             cascasde.push(x.uuid());
        //                         }
        //                     }
        //                     Err(()) => return Err(()),
        //                 },
        //             };
        //         }
        //         self.get_fulfilled().insert(key, target);
        //         for v in cascasde {
        //             match self.fulfill(v) {
        //                 Err(()) => return Err(()),
        //                 Ok(()) => {}
        //             };
        //         }
        //         Ok(())
        //     }
        // }

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
            None => match self.get_fulfilled().remove(&target) {
                // Unknown key in either map.
                None => Err(()),
                Some(mut t) => {
                    t.add_depends_on(depends_on);
                    self.append(t);
                    Ok(())
                }
            },
        }
    }

    fn add_requirement(&mut self, requires: String, is_required: String) -> Result<(), ()> {
        // search for requires
        let mut is_still_required = false;

        match self.get_pending().get_mut(&is_required) {
            Some(is_req) => {
                is_still_required = true;
                is_req.add_required_by(requires.clone());
            }
            None => match self.get_fulfilled().get_mut(&is_required) {
                None => return Err(()),
                Some(is_req) => {
                    is_req.add_required_by(requires.clone());
                }
            },
        };

        match self.get_pending().get_mut(&requires) {
            Some(req) => {
                req.add_depends_on(is_required.clone());
                if !is_still_required {
                    req.depend_fulfilled(is_required).unwrap();
                }
            }

            None => {
                if is_still_required {
                    match self.get_fulfilled().remove(&requires) {
                        None => return Err(()),
                        Some(mut req) => {
                            req.add_depends_on(is_required);
                            self.append(req);
                        }
                    }
                } else {
                    match self.get_fulfilled().get_mut(&requires) {
                        None => return Err(()),
                        Some(req) => {
                            req.add_depends_on(is_required.clone());
                            req.depend_fulfilled(is_required).unwrap();
                        }
                    }
                }
            }
        };

        Ok(())
    }
}

pub struct BasicLattice<T> {
    pending: HashMap<String, T>,
    fulfilled: HashMap<String, T>,
}

impl<T: NodeType> BasicLattice<BasicNode<T>>
where
    T: NodeType,
{
    pub fn from_node_list(v: Vec<BasicNode<T>>) -> BasicLattice<BasicNode<T>> {
        let mut s = BasicLattice::new();
        let mut keys = Vec::<String>::new();

        for node in v {
            if !node.is_pending() {
                keys.push(node.uuid().to_string());
            }

            s.append_pending(node);
        }

        for key in keys {
            s.fulfill(key);
        }

        s
    }
}

// Why doesn't this work?
// impl<T: WriteNode<U>, U> BasicLattice<T>
// where
//     T: WriteNode<U>,
// {
//     pub fn from_node_list(v: Vec<T>) -> BasicLattice<BasicNode<T>> {
//         let mut s = BasicLattice::new();
//         let mut full_map = HashMap::<String, ()>::new();
//         for node in v {
//             full_map.insert(node.uuid(), ());
//             if node.depends_on().is_empty() {
//                 s.append(node);
//             };
//         }
//         s
//     }
// }

impl<T, U> LatMachine<T, U> for BasicLattice<T>
where
    T: WriteNode<U>,
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
