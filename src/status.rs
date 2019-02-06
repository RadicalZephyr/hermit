use std::fmt;

use git2::Repository;

pub struct Status {
    repo: Repository,
}

impl Status {
    pub fn new(repo: Repository) -> Status {
        Status { repo }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "status: ")
    }
}
