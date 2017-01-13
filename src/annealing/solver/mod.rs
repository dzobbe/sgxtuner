use hwloc;
use hwloc::{Topology, ObjectType};
use annealing::problem::Problem;
use annealing::solver::common::MrResult;

pub mod seqsea;
pub mod mips;
pub mod spis;
pub mod prsa;
pub mod common;

pub trait Solver {
    fn solve(&mut self, &mut Problem) -> MrResult;
}
