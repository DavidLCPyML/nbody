use nbody::run;

fn main() {
    pollster::block_on(run());
}