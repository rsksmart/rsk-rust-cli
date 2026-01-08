use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use alloy::primitives::Address;
use std::str::FromStr;

use crate::types::contacts::Contact;
use crate::utils::table::TableBuilder;

#[derive(Parser, Debug)]
pub struct ContactsCommand {
    #[command(subcommand)]
    pub action: ContactsAction,
}

#[derive(Parser, Debug)]
pub enum ContactsAction {
    /// Add a new contact
    Add {
        /// Contact name
        name: String,
        /// Contact address
        address: String,
        /// Notes about the contact
        #[arg(short, long)]
        notes: Option<String>,
        /// Tags to associate with the contact
        #[arg(short, long)]
        tags: Vec<String>,
    },
    /// List all contacts
    List,
    /// Remove a contact
    Remove {
        /// Contact name or address
        identifier: String,
    },
    /// Update a contact
    Update {
        /// Contact name or address
        identifier: String,
        /// New name
        #[arg(long)]
        name: Option<String>,
        /// New address
        #[arg(long)]
        address: Option<String>,
        /// New notes
        #[arg(long)]
        notes: Option<String>,
        /// New tags
        #[arg(long)]
        tags: Option<Vec<String>>,
    },
    /// Get contact details
    Get {
        /// Contact name or address
        identifier: String,
    },
    /// Search contacts
    Search {
        /// Search term
        query: String,
    },
    /// Save contacts to a file
    Save {
        /// File path to save contacts
        file: Option<String>,
    },
    /// Load contacts from a file
    Load {
        /// File path to load contacts from
        file: Option<String>,
    },
}

impl ContactsCommand {
    pub async fn execute(&self) -> Result<()> {
        match &self.action {
            ContactsAction::Add {
                name,
                address,
                notes,
                tags,
            } => {
                self.add_contact(name, address, notes.clone(), tags.clone())
                    .await?
            }
            ContactsAction::List => self.list_contacts().await?,
            ContactsAction::Remove { identifier } => self.remove_contact(identifier).await?,
            ContactsAction::Update {
                identifier,
                name,
                address,
                notes,
                tags,
            } => {
                self.update_contact(
                    identifier,
                    name.clone(),
                    address.clone(),
                    notes.clone(),
                    tags.clone(),
                )
                .await?
            }
            ContactsAction::Get { identifier } => self.get_contact(identifier).await?,
            ContactsAction::Search { query } => self.search_contacts(query).await?,
            ContactsAction::Load { file } => self.load_contacts_from_file(file).await?,
            ContactsAction::Save { file } => self.save_contacts_to_file(file).await?,
        }
        Ok(())
    }

    pub async fn add_contact(
        &self,
        name: &str,
        address: &str,
        notes: Option<String>,
        tags: Vec<String>,
    ) -> Result<()> {
        let address = Address::from_str(address)?;

        let contact = Contact::new(name.to_string(), address, notes, tags);
        contact.validate()?;

        let mut contacts = self.load_contacts()?;
        contacts.push(contact);
        self.save_contacts(&contacts)?;

        println!("{}: Contact added successfully", "Success".green().bold());
        Ok(())
    }

    pub async fn list_contacts(&self) -> Result<()> {
        let contacts = self.load_contacts()?;

        if contacts.is_empty() {
            println!("{}: No contacts found", "Info".yellow().bold());
            return Ok(());
        }

        let mut table = TableBuilder::new();
        table.add_header(&["Name", "Address", "Tags", "Created"]);

        for contact in contacts {
            let tags = if !contact.tags.is_empty() {
                contact.tags.join(", ")
            } else {
                "-".to_string()
            };

            table.add_row(&[
                &contact.name,
                &format!(
                    "{}{}",
                    "0x".green(),
                    contact.address.to_string()[2..].green()
                ),
                &tags,
                &contact.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            ]);
        }

        table.print();
        Ok(())
    }

    pub async fn remove_contact(&self, identifier: &str) -> Result<()> {
        let mut contacts = self.load_contacts()?;

        let index = contacts
            .iter()
            .position(|c| c.name == identifier || c.address.to_string() == identifier)
            .ok_or_else(|| anyhow::anyhow!("Contact not found"))?;

        contacts.remove(index);
        self.save_contacts(&contacts)?;

        println!("{}: Contact removed successfully", "Success".green().bold());
        Ok(())
    }

    //TODO : DEBUG
    pub async fn update_contact(
        &self,
        identifier: &str,
        name: Option<String>,
        address: Option<String>,
        notes: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<()> {
        let mut contacts = self.load_contacts()?;

        let contact = contacts
            .iter_mut()
            .find(|c| c.name == identifier || c.address.to_string() == identifier)
            .ok_or_else(|| anyhow::anyhow!("Contact not found"))?;

        if let Some(name) = name {
            contact.name = name;
        }
        if let Some(address) = address {
            contact.address = address.parse()?;
        }
        if let Some(notes) = notes {
            contact.notes = Some(notes);
        }
        if let Some(tags) = tags {
            contact.tags = tags;
        }

        self.save_contacts(&contacts)?;

        println!("{}: Contact updated successfully", "Success".green().bold());
        Ok(())
    }

    pub async fn get_contact(&self, identifier: &str) -> Result<()> {
        let contacts = self.load_contacts()?;

        let contact = contacts
            .iter()
            .find(|c| c.name == identifier || c.address.to_string() == identifier)
            .ok_or_else(|| anyhow::anyhow!("Contact not found"))?;

        println!("{}", contact);
        Ok(())
    }

    pub async fn search_contacts(&self, query: &str) -> Result<()> {
        let contacts = self.load_contacts()?;

        let matching_contacts: Vec<&Contact> = contacts
            .iter()
            .filter(|c| {
                c.name.to_lowercase().contains(&query.to_lowercase())
                    || c.address.to_string().contains(query)
                    || c.notes.as_ref().is_some_and(|n| n.contains(query))
                    || c.tags.iter().any(|t| t.contains(query))
            })
            .collect();

        if matching_contacts.is_empty() {
            println!(
                "{}: No contacts found matching '{}'",
                "Info".yellow().bold(),
                query
            );
            return Ok(());
        }

        let mut table = TableBuilder::new();
        table.add_header(&["Name", "Address", "Tags", "Created"]);

        for contact in matching_contacts {
            let tags = if !contact.tags.is_empty() {
                contact.tags.join(", ")
            } else {
                "-".to_string()
            };

            table.add_row(&[
                &contact.name,
                &format!(
                    "{}{}",
                    "0x".green(),
                    contact.address.to_string()[2..].green()
                ),
                &tags,
                &contact.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            ]);
        }

        table.print();
        Ok(())
    }

    pub fn load_contacts(&self) -> Result<Vec<Contact>> {
        let contacts_path = dirs::data_local_dir()
            .ok_or_else(|| anyhow::anyhow!("Failed to get data directory"))?
            .join("rsk-rust-cli")
            .join("contacts.json");

        if !contacts_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(contacts_path)?;
        serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse contacts: {}", e))
    }

    // pub fn save_contacts(&self, contacts: &[Contact]) -> Result<()> {
    //     let contacts_path = dirs::data_local_dir()
    //         .ok_or_else(|| anyhow::anyhow!("Failed to get data directory"))?
    //         .join("rsk-rust-cli")
    //         .join("contacts.json");

    //     std::fs::create_dir_all(contacts_path.parent().unwrap())?;
    //     let content = serde_json::to_string_pretty(contacts)?;
    //     std::fs::write(contacts_path, content)?;
    //     Ok(())
    // }
    pub fn save_contacts(&self, contacts: &[Contact]) -> Result<()> {
        let contacts_dir = dirs::data_local_dir()
            .ok_or_else(|| anyhow::anyhow!("Failed to get data directory"))?
            .join("rsk-rust-cli");

        std::fs::create_dir_all(&contacts_dir)?;

        let contacts_path = contacts_dir.join("contacts.json");
        let content = serde_json::to_string_pretty(contacts)?;
        std::fs::write(contacts_path, content)?;
        Ok(())
    }

    pub async fn save_contacts_to_file(&self, file: &Option<String>) -> Result<()> {
        let contacts = self.load_contacts()?;

        let file_path = match file {
            Some(path) => std::path::PathBuf::from(path),
            None => {
                // Default to contacts.json in current directory
                std::env::current_dir()?.join("contacts.json")
            }
        };

        let content = serde_json::to_string_pretty(&contacts)?;
        std::fs::write(&file_path, content)?;

        println!(
            "{}: Contacts saved to {}",
            "Success".green().bold(),
            file_path.display()
        );
        Ok(())
    }

    pub async fn load_contacts_from_file(&self, file: &Option<String>) -> Result<()> {
        let file_path = match file {
            Some(path) => std::path::PathBuf::from(path),
            None => {
                // Default to contacts.json in current directory
                std::env::current_dir()?.join("contacts.json")
            }
        };

        let content = std::fs::read_to_string(&file_path)?;
        let contacts: Vec<Contact> = serde_json::from_str(&content)?;

        // Merge with existing contacts (optional - you might want to replace instead)
        let mut existing_contacts = self.load_contacts().unwrap_or_default();
        existing_contacts.extend(contacts);
        self.save_contacts(&existing_contacts)?;

        println!(
            "{}: Contacts loaded from {}",
            "Success".green().bold(),
            file_path.display()
        );
        Ok(())
    }
}
