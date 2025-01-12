use kyrene_util::TypeIdSet;

pub struct Access {
    pub components: TypeIdSet,
    pub resources: TypeIdSet,
}

impl Access {
    pub fn extend(&mut self, other: Access) {
        self.components.extend(other.components);
        self.resources.extend(other.resources);
    }
}
