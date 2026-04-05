use numnum_core::evaluator::EvalContext;
use numnum_core::format::format_value;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: eval <expression>");
        std::process::exit(1);
    }
    let expr = &args[1];
    let mut ctx = EvalContext::new();
    match ctx.eval_line(expr) {
        Ok(value) => {
            let formatted = format_value(&value, &ctx.unit_table, &ctx.currency_table);
            println!("{}", formatted);
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}
