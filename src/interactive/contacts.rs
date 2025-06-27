use crate::{
    commands::contacts::{ContactsAction, ContactsCommand},
    utils::table::TableBuilder,
};
use anyhow::Result;
use console::style;
use inquire::{Confirm, Text, validator::Validation};
/// Interacive contacts manage
pub async fn manage_contacts() -> Result<()> {
    loop {
        println!("\n{}", style("📇 Contact Management").bold());
        println!("{}", "=".repeat(30));

        let options = vec![
            "👥 List all contacts",
            "➕ Add new contact",
            "✏️  Update contact",
            "❌ Remove contact",
            "🔍 Search contacts",
            "🏠 Back to main menu",
        ];

        let selection = inquire::Select::new("What would you like to do?", options).prompt()?;

        match selection {
            "👥 List all contacts" => list_contacts().await?,
            "➕ Add new contact" => add_contact().await?,
            "✏️  Update contact" => update_contact().await?,
            "❌ Remove contact" => remove_contact().await?,
            "🔍 Search contacts" => search_contacts().await?,
            "🏠 Back to main menu" => break,
            _ => unreachable!(),
        }
    }

    Ok(())
}

/// List all contacts in a table
pub async fn list_contacts() -> Result<()> {
    let mut contacts = ContactsCommand {
        action: ContactsAction::List,
    }
    .load_contacts()?;

    // Sort contacts by most recently interacted with
    contacts.sort_by(|a, b| {
        let a_time = a
            .last_transaction_time()
            .map(|dt| dt.timestamp_millis())
            .unwrap_or(0);
        let b_time = b
            .last_transaction_time()
            .map(|dt| dt.timestamp_millis())
            .unwrap_or(0);
        b_time.cmp(&a_time)
    });

    if contacts.is_empty() {
        println!("No contacts found.");
        return Ok(());
    }

    let mut table = TableBuilder::new();
    table.add_header(&["Name", "Address", "Transactions", "Last Tx"]);

    for contact in contacts {
        let tx_info = if contact.has_transaction_history() {
            format!(
                "{} txs\n{} RBTC",
                contact.get_total_transactions(),
                // Format balance in RBTC (18 decimals)
                ethers::utils::format_units(contact.get_total_volume(), 18)
                    .unwrap_or_else(|_| "N/A".to_string())
            )
        } else {
            "No txs".to_string()
        };

        let last_tx = contact
            .last_transaction_time()
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "Never".to_string());

        table.add_row(&[
            &contact.name,
            &format!("0x{:x}", contact.address),
            &tx_info,
            &last_tx,
        ]);
    }

    table.print();
    Ok(())
}

/// Add a new contact interactively
pub async fn add_contact() -> Result<()> {
    println!("\n{}", style("➕ Add New Contact").bold());

    let name = Text::new("Contact name:")
        .with_help_message("Enter a name for this contact")
        .prompt()?;

    let address = Text::new("Ethereum address (0x...):")
        .with_help_message("Enter the contact's Ethereum address")
        .with_validator(|input: &str| {
            if input.starts_with("0x") && input.len() == 42 {
                Ok(Validation::Valid)
            } else {
                Ok(Validation::Invalid(
                    "Please enter a valid Ethereum address (0x...)".into(),
                ))
            }
        })
        .prompt()?;

    let notes = Text::new("Notes (optional):")
        .with_help_message("Add any notes about this contact")
        .prompt_skippable()?
        .filter(|s| !s.trim().is_empty());

    let tags = Text::new("Tags (comma-separated, optional):")
        .with_help_message("e.g., friend,team,client")
        .prompt_skippable()?
        .map(|s| {
            s.split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let cmd = ContactsCommand {
        action: ContactsAction::Add {
            name,
            address,
            notes,
            tags,
        },
    };

    cmd.execute().await?;
    println!("✅ Contact added successfully!");
    Ok(())
}

/// Update an existing contact
pub async fn update_contact() -> Result<()> {
    let contacts = ContactsCommand {
        action: ContactsAction::List,
    }
    .load_contacts()?;

    if contacts.is_empty() {
        println!("No contacts found to update.");
        return Ok(());
    }

    let contact_names: Vec<String> = contacts
        .iter()
        .map(|c| format!("{} ({})", c.name, c.address))
        .collect();

    let selection = inquire::Select::new("Select contact to update:", contact_names).prompt()?;

    let contact_name = selection.split('(').next().unwrap_or("").trim();

    let new_name = Text::new("New name (press Enter to keep current):")
        .with_help_message("Enter new name or press Enter to skip")
        .prompt_skippable()?;

    let new_address = Text::new("New address (press Enter to keep current):")
        .with_help_message("Enter new address or press Enter to skip")
        .prompt_skippable()?;

    let new_notes = Text::new("New notes (press Enter to keep current):")
        .with_help_message("Enter new notes or press Enter to skip")
        .prompt_skippable()?;

    let new_tags = Text::new("New tags (comma-separated, press Enter to keep current):")
        .with_help_message("e.g., friend,team,client")
        .prompt_skippable()?;

    let cmd = ContactsCommand {
        action: ContactsAction::Update {
            identifier: contact_name.to_string(),
            name: new_name.filter(|s| !s.trim().is_empty()),
            address: new_address.filter(|s| !s.trim().is_empty()),
            notes: new_notes.filter(|s| !s.trim().is_empty()),
            tags: new_tags.map(|s| {
                s.split(',')
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect()
            }),
        },
    };

    cmd.execute().await?;
    println!("✅ Contact updated successfully!");
    Ok(())
}

/// Remove a contact
pub async fn remove_contact() -> Result<()> {
    let contacts = ContactsCommand {
        action: ContactsAction::List,
    }
    .load_contacts()?;

    if contacts.is_empty() {
        println!("No contacts found to remove.");
        return Ok(());
    }

    let contact_names: Vec<String> = contacts
        .iter()
        .map(|c| format!("{} ({})", c.name, c.address))
        .collect();

    let selection = inquire::Select::new("Select contact to remove:", contact_names).prompt()?;

    let contact_name = selection.split('(').next().unwrap_or("").trim();

    if Confirm::new(&format!(
        "Are you sure you want to remove '{}'?",
        contact_name
    ))
    .with_default(false)
    .prompt()?
    {
        let cmd = ContactsCommand {
            action: ContactsAction::Remove {
                identifier: contact_name.to_string(),
            },
        };

        cmd.execute().await?;
        println!("✅ Contact removed successfully!");
    } else {
        println!("Operation cancelled.");
    }

    Ok(())
}

/// Search contacts by name or address
pub async fn search_contacts() -> Result<()> {
    let query = Text::new("Search contacts (name or address):")
        .with_help_message("Enter search term")
        .prompt()?;

    let cmd = ContactsCommand {
        action: ContactsAction::Search {
            query: query.clone(),
        },
    };

    // First try to use the search command's execute
    if let Err(_e) = cmd.execute().await {
        // If execute fails (not implemented), fall back to manual search
        println!("Search not implemented, falling back to local search...");

        let contacts = cmd.load_contacts()?;
        let filtered: Vec<_> = contacts
            .into_iter()
            .filter(|c| {
                c.name.to_lowercase().contains(&query.to_lowercase())
                    || c.address
                        .to_string()
                        .to_lowercase()
                        .contains(&query.to_lowercase())
            })
            .collect();

        if filtered.is_empty() {
            println!("No contacts found matching '{}'", query);
            return Ok(());
        }

        println!("\nFound {} contacts:", filtered.len());
        for contact in filtered {
            println!("• {} - {}", contact.name, contact.address);
        }
    }

    Ok(())
}
