// 1. THE STRUCT (Encapsulation of Data)
// The 'pub' keyword makes this struct visible to other files like main.rs.
pub struct Employee {
    pub name: String,
    pub id: u32,
    salary: f64, // Leaving 'pub' off makes this field PRIVATE (Encapsulation)
}

// 2. THE IMPLEMENTATION BLOCK (Methods & Object Behavior)
impl Employee {
    // This is a "Associated Function" (similar to a Static Constructor / New Class Instance)
    pub fn new(name: &str, id: u32, starting_salary: f64) -> Self {
        Self {
            name: name.to_string(), // Convert the string reference to an owned String
            id,
            salary: starting_salary,
        }
    }

    // A standard Method. '&self' means itBORROWS data from the struct instance.
    pub fn display_info(&self) {
        println!("ID: {} | Name: {}", self.id, self.name);
    }

    // A mutable method. '&mut self' allows modifying internal private properties safely.
    pub fn give_raise(&mut self, amount: f64) {
        if amount > 0.0 {
            self.salary += amount;
            println!(
                "{} received a raise! New private salary verified.",
                self.name
            );
        }
    }
}

// 3. THE TRAIT (Polymorphism / Interfaces)
// Rust doesn't use inheritance. It uses Traits to define shared behavior.
pub trait Reportable {
    fn generate_report(&self) -> String;
}

// Implement the Trait behavior specifically for our Employee Struct
impl Reportable for Employee {
    fn generate_report(&self) -> String {
        format!(
            "REPORT -> Employee #{} ({}) is in good standing.",
            self.id, self.name
        )
    }
}

// This module only compiles when you run 'cargo test'
#[cfg(test)]
mod tests {
    use super::*; // Import everything from the outer employee module

    #[test]
    fn test_employee_creation() {
        let emp = Employee::new("Test Admin", 999, 50000.0);
        assert_eq!(emp.name, "Test Admin");
        assert_eq!(emp.id, 999);
    }

    #[test]
    fn test_give_raise() {
        let mut emp = Employee::new("Alex", 1002, 60000.0);
        emp.give_raise(5000.0);
        // Even though salary is private, tests in the same file can check it!
        assert_eq!(emp.salary, 65000.0);
    }
}
