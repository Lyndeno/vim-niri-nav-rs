use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(about = "Navigate between vim splits and niri windows")]
pub struct Args {
    pub direction: Direction,
    pub modifier: Option<Modifier>,
}

#[derive(ValueEnum, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(ValueEnum, Clone, Copy)]
pub enum Modifier {
    #[value(name = "w")]
    Workspace,
    #[value(name = "m")]
    Monitor,
}
