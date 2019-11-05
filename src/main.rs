#![feature(test)]

mod tracemap;

use hwtracer::backends::{TracerBuilder};
use tracemap::{MapTrace};

fn main() {

    let mt = MapTrace::new();

    let tracer = TracerBuilder::new().build().unwrap();
    let mut ttracer = (*tracer).thread_tracer();
    ttracer.start_tracing();
    execute_block(10);
    let trace = ttracer.stop_tracing().unwrap();

    let annotrace = mt.annotate_trace(trace).unwrap();

    for (taddr, lbl) in annotrace {
        println!("{:x?}: {}", taddr, lbl);
    }
}

fn execute_block(x: usize) -> usize{
    let a = 1;
    empty_block();
    let mut c = sub_block(true); // bb0
    let mut i = 0; // bb1
    while i < x { // bb2
        c += 2; // bb4-6
        i += 1;
    }
    c //bb3
}

fn empty_block() {
}

#[inline(never)]
fn sub_block(c: bool) -> usize {
    if c {
        return 5000000000;
    }
    else {
        return 1000000000;
    }
}

