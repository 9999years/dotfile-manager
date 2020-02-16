use dialoguer::{theme::ColorfulTheme, Checkboxes, Confirmation, Input, OrderList, Select};

fn main() {
    println!("Example using dialoguer::Input with dialoguer::Validator:");
    let input0 = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Your email address")
        .validate_with(|txt: &str| if txt.contains('@') { Ok(()) } else { Err(
            "An email address must contain an @"
        ) })
        .interact()
        .unwrap();
    println!("{:?}", input0);

    println!("Checkboxes.");
    let input = Checkboxes::with_theme(&ColorfulTheme::default())
        .items(&["run", "info", "install"])
        .interact()
        .unwrap();
    println!("{:?}", input);
    println!("Confirmation.");
    let input2 = Confirmation::with_theme(&ColorfulTheme::default())
        .with_text("Continue?")
        .interact()
        .unwrap();
    println!("{:?}", input2);

    println!("Select.");
    let input3 = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a programming language")
        .items(&["Rust", "Python", "Haskell", "Java"])
        .interact()
        .unwrap();
    println!("{:?}", input3);

    println!("OrderList");
    let input4 = OrderList::with_theme(&ColorfulTheme::default())
        .with_prompt("Rank these drinks")
        .items(&["Rust", "Python", "Haskell", "Java"])
        .interact()
        .unwrap();
    println!("{:?}", input4);
}
