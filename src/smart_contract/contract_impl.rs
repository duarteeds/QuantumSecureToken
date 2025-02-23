use std::error::Error as StdError;
use wasmer::{imports, Instance, Module, Store, Value};
use serde::{Serialize, Deserialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartContract {
    pub code: Vec<u8>,
    pub address: String,
    pub quantum_secure: bool,
}

impl SmartContract {
    pub fn new(code: Vec<u8>, address: String, quantum_secure: bool) -> Self {
        SmartContract {
            code,
            address,
            quantum_secure,
        }
    }

    pub fn execute(&self, input: &str) -> Result<String, Box<dyn StdError>> {
        if self.quantum_secure {
            let store = Store::default();
            let module = Module::new(&store, &self.code)?;
            let import_object = imports! {};
            let instance = Instance::new(&module, &import_object)?;
            let main_func = instance.exports.get_function("main")?;
            let args = [Value::I32(input.len() as i32)];
            let result = main_func.call(&args)?;
            Ok(format!("{:?}", result))
        } else {
            Ok("Execução não quantificada".to_string())
        }
    }
}