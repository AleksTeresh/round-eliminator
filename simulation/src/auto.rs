type Problem = crate::problem::GenericProblem;

/// A chain of simplifications.
/// We start from an initial problem,
/// then at each step we can either simplify and get a new problem,
/// or perform one step of speedup and get a new problem
#[derive(Clone, Debug)]
pub enum Step<T: Clone + std::fmt::Debug> {
    Initial(Problem),
    Simplify((T, Problem)),
    Speedup(Problem),
    MergeEqual(Problem)
}

/// A generic simplification strategy should implement this trait.
/// A strategy should provide a list of possible simplifications that can be done starting from the current step,
/// and it should be able to tell if the current state is better than the current best one,
/// and if it makes sense to continue trying the current path.
/// Also, it needs to provide a way to simplify the current problem, given the current simplification.
pub trait Auto: Sized + Clone {
    type Simplification: Clone + std::fmt::Debug;
    /// constructor
    fn new(features: &[&str]) -> Self;
    /// given the current state and the maximum number of labels, returns an iterator over the possible simplifications that can be performed.
    fn simplifications(
        &mut self,
        sequence: &mut Sequence<Self>,
        maxlabels: usize,
    ) -> Box<dyn Iterator<Item = Self::Simplification>>;
    /// given the current state, the current best state, and the maximum number of speedup steps, returns true if the current state is better than the stored best one.
    fn should_yield(
        &mut self,
        sequence: &mut Sequence<Self>,
        best: &mut Sequence<Self>,
        maxiter: usize
    ) -> bool;
    /// given the current state, the current best state, and the maximum number of speedup steps, returns true it makes sense to do more speedup steps.
    fn should_continue(
        &mut self,
        sequence: &mut Sequence<Self>,
        best: &mut Sequence<Self>,
        maxiter: usize
    ) -> bool;
    /// given a problem (sequence.current()) and a simplification, return a new problem where the simplification has been performed.
    /// If for some reason the simplification does not make sense anymore, return None.
    fn simplify(
        &mut self,
        sequence: &mut Sequence<Self>,
        simpl: Self::Simplification,
    ) -> Option<Problem>;
}

#[derive(Clone)]
pub struct Sequence<T>
where
    T: Auto,
{
    pub steps: Vec<Step<T::Simplification>>,
    pub speedups: usize,
}

impl<T> Sequence<T>
where
    T: Auto,
{
    pub fn new(p: Problem) -> Self {
        Self {
            steps: vec![Step::Initial(p)],
            speedups: 0,
        }
    }

    pub fn current(&self) -> &Problem {
        match self.steps.last().unwrap() {
            Step::Initial(p) => p,
            Step::Simplify((_, p)) => p,
            Step::Speedup(p) => p,
            Step::MergeEqual(p) => p
        }
    }

    pub fn current_mut(&mut self) -> &mut Problem {
        match self.steps.last_mut().unwrap() {
            Step::Initial(p) => p,
            Step::Simplify((_, p)) => p,
            Step::Speedup(p) => p,
            Step::MergeEqual(p) => p
        }
    }

    fn make_printable(&mut self) {
        for step in self.steps.iter_mut() {
            match step {
                Step::Initial(p) => {
                    let _ = p.as_result();
                }
                Step::Simplify((_, p)) => {
                    let _ = p.as_result();
                }
                Step::Speedup(p) => {
                    let _ = p.as_result();
                }
                Step::MergeEqual(p) => {
                    let _ = p.as_result();
                }
            }
        }
    }

    fn push(&mut self, step: Step<T::Simplification>) {
        self.steps.push(step);
    }

    fn pop(&mut self) {
        self.steps.pop();
    }

    fn pop_speedup(&mut self) {
        self.speedups -= 1;
        self.pop_mergeequal();
        self.pop();
    }

    #[must_use]
    fn push_speedup(&mut self) -> Result<(), String> {
        self.speedups += 1;
        let last = self.current_mut();
        let new = last.speedup()?;
        self.push(Step::Speedup(new));
        self.merge_equal();
        Ok(())
    }

    fn push_simplification(&mut self, simpl: T::Simplification, auto: &mut T) -> bool {
        if let Some(new) = auto.simplify(self, simpl.clone()) {
            self.push(Step::Simplify((simpl, new)));
            self.merge_equal();
            return true;
        }
        false
    }

    fn pop_simplification(&mut self) {
        self.pop_mergeequal();
        self.pop();
    }

    fn merge_equal(&mut self){
        while !self.current().mergeable.as_ref().map(|t|t.is_empty()).unwrap_or(true) {
            let new = self.current().merge_equal();
            self.push(Step::MergeEqual(new));
        }
    }

    fn pop_mergeequal(&mut self){
        while let Step::MergeEqual(_) = self.steps.last().unwrap() {
            self.pop();
        }
    }

}

pub struct AutomaticSimplifications<T: Auto> {
    pub sol: Sequence<T>,
    pub best: Sequence<T>,
    pub maxiter: usize,
    pub maxlabels: usize,
    pub maxrcs: usize,
    auto: T,
}

impl<T: Auto> AutomaticSimplifications<T> {
    pub fn new(
        p: Problem,
        maxiter: usize,
        maxlabels: usize,
        maxrcs: usize,
        features: &[&str],
    ) -> Self {
        let sol = Sequence::new(p);
        let best = sol.clone();
        Self {
            sol,
            best,
            maxiter,
            maxlabels,
            maxrcs,
            auto: T::new(features),
        }
    }

    /// internal iterator version of automatic simplification,
    /// each time a better result is found, the closure is called
    #[allow(dead_code)]
    pub fn run<F>(&mut self, mut cb: F) -> Result<(), String>
    where
        F: FnMut(&Sequence<T>),
    {
        if self.sol.current().is_zero_rounds() {
            self.sol.make_printable();
            cb(&self.sol);
        }
        self.problem(&mut cb)?;
        Ok(())
    }

    #[must_use]
    fn problem<F>(&mut self, cb: &mut F) -> Result<(), String>
    where
        F: FnMut(&Sequence<T>),
    {
        if self
            .auto
            .should_yield(&mut self.sol, &mut self.best, self.maxiter)
        {
            self.best = self.sol.clone();
            self.best.make_printable();
            cb(&self.best);
        }
        if self
            .auto
            .should_continue(&mut self.sol, &mut self.best, self.maxiter)
        {
            self.simplify(cb)?;
        }
        Ok(())
    }

    #[must_use]
    fn simplify<F>(&mut self, cb: &mut F) -> Result<(), String>
    where
        F: FnMut(&Sequence<T>),
    {
        if self.sol.current().num_labels() <= self.maxlabels && self.sol.current().right_closed_subsets().len() <= self.maxrcs {
            self.sol.push_speedup()?;
            self.problem(cb)?;
            self.sol.pop_speedup();
        } else {
            for simpl in self.auto.simplifications(&mut self.sol, self.maxlabels) {
                if self.sol.push_simplification(simpl, &mut self.auto) {
                    self.simplify(cb)?;
                    self.sol.pop_simplification();
                }
            }
        }
        Ok(())
    }
}

impl<T: Auto> IntoIterator for AutomaticSimplifications<T> {
    type Item = Result<Sequence<T>, String>;
    type IntoIter = AutomaticSimplificationsIntoIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        AutomaticSimplificationsIntoIterator {
            auto: self,
            stack: vec![State::Start],
        }
    }
}

/// External iterator version of automatic simplification.
/// This allows to get a proper rust iterator, but the code is ugly,
/// since the recursion needs to be converted to a state machine.
enum State<T: Auto> {
    Start,
    Problem,
    ProblemAfterCheckYield,
    Simplify,
    SimplifyAfterProblemCall,
    SimplifyAfterSimplifyCall,
    SimplifySimplify(Box<dyn Iterator<Item = T::Simplification>>),
    Error,
}

pub struct AutomaticSimplificationsIntoIterator<T: Auto> {
    auto: AutomaticSimplifications<T>,
    stack: Vec<State<T>>,
}

impl<T: Auto> Iterator for AutomaticSimplificationsIntoIterator<T> {
    type Item = Result<Sequence<T>, String>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.stack.is_empty() {
                return None;
            }
            match self.stack.last_mut().unwrap() {
                State::Start => {
                    self.stack.pop();
                    if self.auto.sol.current().is_zero_rounds() {
                        self.auto.sol.make_printable();
                        return Some(Ok(self.auto.sol.clone()));
                    }
                    self.stack.push(State::Problem);
                }
                State::Problem => {
                    self.stack.pop();
                    self.stack.push(State::ProblemAfterCheckYield);
                    if self.auto.auto.should_yield(
                        &mut self.auto.sol,
                        &mut self.auto.best,
                        self.auto.maxiter
                    ) {
                        self.auto.best = self.auto.sol.clone();
                        self.auto.best.make_printable();
                        return Some(Ok(self.auto.best.clone()));
                    }
                }
                State::ProblemAfterCheckYield => {
                    self.stack.pop();
                    if self.auto.auto.should_continue(
                        &mut self.auto.sol,
                        &mut self.auto.best,
                        self.auto.maxiter
                    ) {
                        self.stack.push(State::Simplify);
                    }
                }
                State::Simplify => {
                    self.stack.pop();
                    if self.auto.sol.current().num_labels() <= self.auto.maxlabels && self.auto.sol.current().right_closed_subsets().len() <= self.auto.maxrcs {
                        if let Err(s) = self.auto.sol.push_speedup() {
                            self.stack.push(State::Error);
                            return Some(Err(s));
                        }
                        self.stack.push(State::SimplifyAfterProblemCall);
                        self.stack.push(State::Problem);
                    } else {
                        self.stack.push(State::SimplifySimplify(
                            self.auto
                                .auto
                                .simplifications(&mut self.auto.sol, self.auto.maxlabels),
                        ));
                    }
                }
                State::SimplifyAfterProblemCall => {
                    self.auto.sol.pop_speedup();
                    self.stack.pop();
                }
                State::SimplifySimplify(iter) => {
                    if let Some(simpl) = iter.next() {
                        if self
                            .auto
                            .sol
                            .push_simplification(simpl, &mut self.auto.auto)
                        {
                            self.stack.push(State::SimplifyAfterSimplifyCall);
                            self.stack.push(State::Simplify);
                        }
                    } else {
                        self.stack.pop();
                    }
                }
                State::SimplifyAfterSimplifyCall => {
                    self.auto.sol.pop_simplification();
                    self.stack.pop();
                }
                State::Error => {
                    self.stack.clear();
                }
            }
        }
    }
}
