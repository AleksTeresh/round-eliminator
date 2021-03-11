use simulation::AutoUb;
use simulation::AutomaticSimplifications;
use simulation::GenericProblem;
use simulation::Config;
use simulation::do_multiple_speedups;

fn do_pp_search(
    data: String,
    config: Config,
    iter: usize,
    merge: bool
) -> (usize, bool, bool) {
    let p = GenericProblem::from_line_separated_text(&data, config).unwrap();
    let (res, found_periodic_point, found_zero_round) = do_multiple_speedups(p, iter, merge, true);
    (res.len(), found_periodic_point, found_zero_round)
}

fn do_autoub(
    data: String,
    config: Config,
    labels: usize,
    iter: usize,
    autoub_features: String
) -> i32 {
    let p = GenericProblem::from_line_separated_text(&data, config).unwrap();
    let autoub_features : Vec<_> = autoub_features.split(",").collect();
    let auto = AutomaticSimplifications::<AutoUb>::new(
        p,
        iter,
        labels,
        1000,
        &autoub_features
    );
    let mut res: i32 = -1;
    for x in auto {
        let sol = x.unwrap();
        let local_res = sol.speedups as i32;
        if res == -1 || local_res < res {
            res = local_res;
        }
    }
    res
}

pub fn search_for_complexity_sequential(
  data: String,
  config: Config,
  labels: usize,
  iter: usize,
  merge: bool,
  autoub_features: String,
  periodic_point_only: bool
) -> (String, String) {
    let mut lower_bound = String::from("unknown");
    let mut upper_bound = String::from("unknown");

    /////////////////
    let (round_count,
        found_periodic_point,
        found_zero_round
    ) = do_pp_search(data.clone(), config, iter, merge);

    if found_periodic_point && !found_zero_round {
        lower_bound = String::from("log n");
        return (lower_bound, upper_bound);
    }
    if found_zero_round {
        lower_bound = round_count.to_string();
        upper_bound = round_count.to_string();
        return (lower_bound, upper_bound);
    }

///////////////////////////////

    if periodic_point_only {
        return (lower_bound, upper_bound);
    }

///////////////////////////////

    let upper_bound_res = do_autoub(data.clone(), config, labels, iter, autoub_features);
    if upper_bound_res != -1 && upper_bound == "unknown" {
        upper_bound = upper_bound_res.to_string();
    }

    (lower_bound, upper_bound)
}
