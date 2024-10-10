// Copied with minor modifications from blockifier/src/execution/stack_trace.rs

use blockifier::execution::deprecated_syscalls::hint_processor::DeprecatedSyscallExecutionError;
use blockifier::execution::errors::{
    ConstructorEntryPointExecutionError, EntryPointExecutionError,
};
use blockifier::execution::syscalls::hint_processor::SyscallExecutionError;
use blockifier::transaction::errors::TransactionExecutionError;
use cairo_vm::types::relocatable::Relocatable;
use cairo_vm::vm::errors::cairo_run_errors::CairoRunError;
use cairo_vm::vm::errors::hint_errors::HintError;
use cairo_vm::vm::errors::vm_errors::VirtualMachineError;
use starknet_api::core::{ClassHash, ContractAddress, EntryPointSelector};

pub const TRACE_LENGTH_CAP: usize = 15000;
pub const TRACE_EXTRA_CHARS_SLACK: usize = 100;

#[derive(serde::Serialize, serde::Deserialize)]
pub enum PreambleType {
    CallContract,
    LibraryCall,
    Constructor,
}

impl PreambleType {
    pub fn text(&self) -> &str {
        match self {
            Self::CallContract => "Error in the called contract",
            Self::LibraryCall => "Error in a library call",
            Self::Constructor => "Error in the contract class constructor",
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct EntryPointErrorFrame {
    pub depth: usize,
    pub preamble_type: PreambleType,
    pub storage_address: ContractAddress,
    pub class_hash: ClassHash,
    pub selector: Option<EntryPointSelector>,
}

impl EntryPointErrorFrame {
    fn preamble_text(&self) -> String {
        format!(
            "{}: {} (contract address: {:#064x}, class hash: {:#064x}, selector: {}):",
            self.depth,
            self.preamble_type.text(),
            self.storage_address.0.key(),
            self.class_hash.0,
            if let Some(selector) = self.selector {
                format!("{:#064x}", selector.0)
            } else {
                "UNKNOWN".to_string()
            }
        )
    }
}

impl From<&EntryPointErrorFrame> for String {
    fn from(value: &EntryPointErrorFrame) -> Self {
        value.preamble_text()
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct VmExceptionFrame {
    pub pc: Relocatable,
    pub error_attr_value: Option<String>,
    pub traceback: Option<String>,
}

impl From<&VmExceptionFrame> for String {
    fn from(value: &VmExceptionFrame) -> Self {
        let error_msg = match &value.error_attr_value {
            Some(error_msg) => error_msg.clone(),
            None => String::new(),
        };
        let vm_exception_preamble = format!("Error at pc={}:", value.pc);
        let vm_exception_traceback = if let Some(traceback) = &value.traceback {
            format!("\n{}", traceback)
        } else {
            "".to_string()
        };
        format!("{error_msg}{vm_exception_preamble}{vm_exception_traceback}")
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum Frame {
    EntryPoint(EntryPointErrorFrame),
    Vm(VmExceptionFrame),
    StringFrame(String),
}

impl From<&Frame> for String {
    fn from(value: &Frame) -> Self {
        match value {
            Frame::EntryPoint(entry_point_frame) => entry_point_frame.into(),
            Frame::Vm(vm_exception_frame) => vm_exception_frame.into(),
            Frame::StringFrame(error) => error.clone(),
        }
    }
}

impl From<EntryPointErrorFrame> for Frame {
    fn from(value: EntryPointErrorFrame) -> Self {
        Frame::EntryPoint(value)
    }
}

impl From<VmExceptionFrame> for Frame {
    fn from(value: VmExceptionFrame) -> Self {
        Frame::Vm(value)
    }
}

impl From<String> for Frame {
    fn from(value: String) -> Self {
        Frame::StringFrame(value)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct ErrorStack {
    pub stack: Vec<Frame>,
}

impl ErrorStack {
    pub fn push(&mut self, frame: Frame) {
        self.stack.push(frame);
    }
}

/// Extracts the error trace from a `TransactionExecutionError`. This is a top level function.
pub fn gen_tx_execution_error_trace(error: &TransactionExecutionError) -> ErrorStack {
    match error {
        TransactionExecutionError::ExecutionError {
            error,
            class_hash,
            storage_address,
            selector,
        }
        | TransactionExecutionError::ValidateTransactionError {
            error,
            class_hash,
            storage_address,
            selector,
        } => gen_error_trace_from_entry_point_error(
            error,
            storage_address,
            class_hash,
            Some(selector),
            PreambleType::CallContract,
        ),
        TransactionExecutionError::ContractConstructorExecutionFailed(
            ConstructorEntryPointExecutionError::ExecutionError {
                error,
                class_hash,
                contract_address: storage_address,
                constructor_selector,
            },
        ) => gen_error_trace_from_entry_point_error(
            error,
            storage_address,
            class_hash,
            constructor_selector.as_ref(),
            PreambleType::Constructor,
        ),
        _ => {
            // Top-level error is unrelated to Cairo execution, no "real" frames.
            let mut stack = ErrorStack::default();
            stack.push(Frame::StringFrame(error.to_string()));
            stack
        }
    }
}

/// Generate error stack from top-level entry point execution error.
fn gen_error_trace_from_entry_point_error(
    error: &EntryPointExecutionError,
    storage_address: &ContractAddress,
    class_hash: &ClassHash,
    entry_point_selector: Option<&EntryPointSelector>,
    preamble_type: PreambleType,
) -> ErrorStack {
    let mut error_stack: ErrorStack = ErrorStack::default();
    let depth = 0;
    error_stack.push(
        EntryPointErrorFrame {
            depth,
            preamble_type,
            storage_address: *storage_address,
            class_hash: *class_hash,
            selector: entry_point_selector.copied(),
        }
        .into(),
    );
    extract_entry_point_execution_error_into_stack_trace(&mut error_stack, depth + 1, error);
    error_stack
}

fn extract_cairo_run_error_into_stack_trace(
    error_stack: &mut ErrorStack,
    depth: usize,
    error: &CairoRunError,
) {
    if let CairoRunError::VmException(vm_exception) = error {
        error_stack.push(
            VmExceptionFrame {
                pc: vm_exception.pc,
                error_attr_value: vm_exception.error_attr_value.clone(),
                traceback: vm_exception.traceback.clone(),
            }
            .into(),
        );
        extract_virtual_machine_error_into_stack_trace(error_stack, depth, &vm_exception.inner_exc);
    } else {
        error_stack.push(error.to_string().into());
    }
}

fn extract_virtual_machine_error_into_stack_trace(
    error_stack: &mut ErrorStack,
    depth: usize,
    vm_error: &VirtualMachineError,
) {
    match vm_error {
        VirtualMachineError::Hint(ref boxed_hint_error) => {
            if let HintError::Internal(internal_vm_error) = &boxed_hint_error.1 {
                return extract_virtual_machine_error_into_stack_trace(
                    error_stack,
                    depth,
                    internal_vm_error,
                );
            }
            error_stack.push(boxed_hint_error.1.to_string().into());
        }
        VirtualMachineError::Other(anyhow_error) => {
            let syscall_exec_err = anyhow_error.downcast_ref::<SyscallExecutionError>();
            if let Some(downcast_anyhow) = syscall_exec_err {
                extract_syscall_execution_error_into_stack_trace(
                    error_stack,
                    depth,
                    downcast_anyhow,
                )
            } else {
                let deprecated_syscall_exec_err =
                    anyhow_error.downcast_ref::<DeprecatedSyscallExecutionError>();
                if let Some(downcast_anyhow) = deprecated_syscall_exec_err {
                    extract_deprecated_syscall_execution_error_into_stack_trace(
                        error_stack,
                        depth,
                        downcast_anyhow,
                    )
                }
            }
        }
        _ => {
            error_stack.push(format!("{}\n", vm_error).into());
        }
    }
}

fn extract_syscall_execution_error_into_stack_trace(
    error_stack: &mut ErrorStack,
    depth: usize,
    syscall_error: &SyscallExecutionError,
) {
    match syscall_error {
        SyscallExecutionError::CallContractExecutionError {
            class_hash,
            storage_address,
            selector,
            error,
        } => {
            error_stack.push(
                EntryPointErrorFrame {
                    depth,
                    preamble_type: PreambleType::CallContract,
                    storage_address: *storage_address,
                    class_hash: *class_hash,
                    selector: Some(*selector),
                }
                .into(),
            );
            extract_syscall_execution_error_into_stack_trace(error_stack, depth + 1, error)
        }
        SyscallExecutionError::LibraryCallExecutionError {
            class_hash,
            storage_address,
            selector,
            error,
        } => {
            error_stack.push(
                EntryPointErrorFrame {
                    depth,
                    preamble_type: PreambleType::LibraryCall,
                    storage_address: *storage_address,
                    class_hash: *class_hash,
                    selector: Some(*selector),
                }
                .into(),
            );
            extract_syscall_execution_error_into_stack_trace(error_stack, depth + 1, error);
        }
        SyscallExecutionError::ConstructorEntryPointExecutionError(
            ConstructorEntryPointExecutionError::ExecutionError {
                error,
                class_hash,
                contract_address,
                constructor_selector,
            },
        ) => {
            error_stack.push(
                EntryPointErrorFrame {
                    depth,
                    preamble_type: PreambleType::Constructor,
                    storage_address: *contract_address,
                    class_hash: *class_hash,
                    selector: *constructor_selector,
                }
                .into(),
            );
            extract_entry_point_execution_error_into_stack_trace(error_stack, depth, error)
        }
        SyscallExecutionError::EntryPointExecutionError(entry_point_error) => {
            extract_entry_point_execution_error_into_stack_trace(
                error_stack,
                depth,
                entry_point_error,
            )
        }
        _ => {
            error_stack.push(syscall_error.to_string().into());
        }
    }
}

fn extract_deprecated_syscall_execution_error_into_stack_trace(
    error_stack: &mut ErrorStack,
    depth: usize,
    syscall_error: &DeprecatedSyscallExecutionError,
) {
    match syscall_error {
        DeprecatedSyscallExecutionError::CallContractExecutionError {
            class_hash,
            storage_address,
            selector,
            error,
        } => {
            error_stack.push(
                EntryPointErrorFrame {
                    depth,
                    preamble_type: PreambleType::CallContract,
                    storage_address: *storage_address,
                    class_hash: *class_hash,
                    selector: Some(*selector),
                }
                .into(),
            );
            extract_deprecated_syscall_execution_error_into_stack_trace(
                error_stack,
                depth + 1,
                error,
            )
        }
        DeprecatedSyscallExecutionError::LibraryCallExecutionError {
            class_hash,
            storage_address,
            selector,
            error,
        } => {
            error_stack.push(
                EntryPointErrorFrame {
                    depth,
                    preamble_type: PreambleType::LibraryCall,
                    storage_address: *storage_address,
                    class_hash: *class_hash,
                    selector: Some(*selector),
                }
                .into(),
            );
            extract_deprecated_syscall_execution_error_into_stack_trace(
                error_stack,
                depth + 1,
                error,
            )
        }
        DeprecatedSyscallExecutionError::ConstructorEntryPointExecutionError(
            ConstructorEntryPointExecutionError::ExecutionError {
                error,
                class_hash,
                contract_address,
                constructor_selector,
            },
        ) => {
            error_stack.push(
                EntryPointErrorFrame {
                    depth,
                    preamble_type: PreambleType::Constructor,
                    storage_address: *contract_address,
                    class_hash: *class_hash,
                    selector: *constructor_selector,
                }
                .into(),
            );
            extract_entry_point_execution_error_into_stack_trace(error_stack, depth, error)
        }
        DeprecatedSyscallExecutionError::EntryPointExecutionError(entry_point_error) => {
            extract_entry_point_execution_error_into_stack_trace(
                error_stack,
                depth,
                entry_point_error,
            )
        }
        _ => error_stack.push(syscall_error.to_string().into()),
    }
}

fn extract_entry_point_execution_error_into_stack_trace(
    error_stack: &mut ErrorStack,
    depth: usize,
    entry_point_error: &EntryPointExecutionError,
) {
    match entry_point_error {
        EntryPointExecutionError::CairoRunError(cairo_run_error) => {
            extract_cairo_run_error_into_stack_trace(error_stack, depth, cairo_run_error)
        }
        _ => error_stack.push(format!("{}\n", entry_point_error).into()),
    }
}
