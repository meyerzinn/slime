use slime::run;

fn main() {
    env_logger::init();
    
    pollster::block_on(run());
}
