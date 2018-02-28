use annealing::problem::Problem;
use annealing::solver::common::MrResult;

pub mod seqsa;
pub mod mir;
pub mod spisa;
//pub mod prsa;
pub mod common;

pub trait Solver {
    fn solve(&mut self, &mut Problem, usize) -> MrResult;
}
