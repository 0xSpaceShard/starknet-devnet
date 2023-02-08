"""Patch starknet methods to use cairo_rs_py"""

# pylint: disable=bare-except
# pylint: disable=missing-function-docstring
# pylint: disable=protected-access
# pylint: disable=too-many-locals


import logging
import sys
from typing import Any, Callable, Dict, List, Optional, Tuple, Type, Union, cast

import cairo_rs_py
from cairo_rs_py import RelocatableValue  # pylint: disable = no-name-in-module
from starkware.cairo.common.cairo_function_runner import CairoFunctionRunner
from starkware.cairo.common.structs import CairoStructFactory
from starkware.cairo.lang.compiler.ast.cairo_types import (
    CairoType,
    TypeFelt,
    TypePointer,
)
from starkware.cairo.lang.compiler.program import Program
from starkware.cairo.lang.compiler.scoped_name import ScopedName
from starkware.cairo.lang.vm.memory_segments import MemorySegmentManager
from starkware.cairo.lang.vm.relocatable import MaybeRelocatable
from starkware.cairo.lang.vm.utils import ResourcesError
from starkware.cairo.lang.vm.vm_exceptions import (
    SecurityError,
    VmException,
    VmExceptionBase,
)
from starkware.python.utils import safe_zip
from starkware.starknet.business_logic.execution.execute_entry_point import (
    FAULTY_CLASS_HASH,
    ExecuteEntryPoint,
)
from starkware.starknet.business_logic.execution.objects import (
    TransactionExecutionContext,
)
from starkware.starknet.business_logic.fact_state.state import ExecutionResourcesManager
from starkware.starknet.business_logic.state.state_api import SyncState
from starkware.starknet.business_logic.utils import validate_contract_deployed
from starkware.starknet.core.os import os_utils, segment_utils, syscall_utils
from starkware.starknet.core.os.class_hash import (
    get_contract_class_struct,
    load_program,
)
from starkware.starknet.definitions.error_codes import StarknetErrorCode
from starkware.starknet.definitions.general_config import StarknetGeneralConfig
from starkware.starknet.public import abi as starknet_abi
from starkware.starknet.public.abi import SYSCALL_PTR_OFFSET
from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starkware_utils.error_handling import (
    ErrorCode,
    StarkException,
    stark_assert,
    wrap_with_stark_exception,
)

logger = logging.getLogger(__name__)


def cairo_rs_py_run(
    self,
    state: SyncState,
    resources_manager: ExecutionResourcesManager,
    general_config: StarknetGeneralConfig,
    tx_execution_context: TransactionExecutionContext,
) -> Tuple[CairoFunctionRunner, syscall_utils.BusinessLogicSysCallHandler]:
    """
    Runs the selected entry point with the given calldata in the code of the contract deployed
    at self.code_address.
    The execution is done in the context (e.g., storage) of the contract at
    self.contract_address.
    Returns the corresponding CairoFunctionRunner and BusinessLogicSysCallHandler in order to
    retrieve the execution information.
    """
    # Prepare input for Cairo function runner.
    class_hash = self._get_code_class_hash(state=state)

    # Hack to prevent version 0 attack on argent accounts.
    if (tx_execution_context.version == 0) and (class_hash == FAULTY_CLASS_HASH):
        raise StarkException(
            code=StarknetErrorCode.TRANSACTION_FAILED, message="Fraud attempt blocked."
        )

    contract_class = state.get_contract_class(class_hash=class_hash)
    contract_class.validate()

    entry_point = self._get_selected_entry_point(
        contract_class=contract_class, class_hash=class_hash
    )

    # Run the specified contract entry point with given calldata.
    with wrap_with_stark_exception(code=StarknetErrorCode.SECURITY_ERROR):
        runner = cairo_rs_py.CairoRunner(  # pylint: disable=no-member
            program=contract_class.program.dumps(),
            entrypoint=None,
            layout="all",
            proof_mode=False,
        )
        runner.initialize_function_runner()
    os_context = os_utils.prepare_os_context(runner=runner)

    validate_contract_deployed(state=state, contract_address=self.contract_address)

    initial_syscall_ptr = cast(
        RelocatableValue, os_context[starknet_abi.SYSCALL_PTR_OFFSET]
    )
    syscall_handler = syscall_utils.BusinessLogicSysCallHandler(
        execute_entry_point_cls=ExecuteEntryPoint,
        tx_execution_context=tx_execution_context,
        state=state,
        resources_manager=resources_manager,
        caller_address=self.caller_address,
        contract_address=self.contract_address,
        general_config=general_config,
        initial_syscall_ptr=initial_syscall_ptr,
    )

    # Positional arguments are passed to *args in the 'run_from_entrypoint' function.
    entry_points_args = [
        self.entry_point_selector,
        os_context,
        len(self.calldata),
        # Allocate and mark the segment as read-only (to mark every input array as read-only).
        syscall_handler._allocate_segment(segments=runner.segments, data=self.calldata),
    ]

    try:
        runner.run_from_entrypoint(
            entry_point.offset,
            entry_points_args,
            hint_locals={
                "syscall_handler": syscall_handler,
            },
            static_locals={
                "__find_element_max_size": 2**20,
                "__squash_dict_max_size": 2**20,
                "__keccak_max_size": 2**20,
                "__usort_max_size": 2**20,
                "__chained_ec_op_max_len": 1000,
            },
            run_resources=tx_execution_context.run_resources,
            verify_secure=True,
        )
    except VmException as exception:
        code: ErrorCode = StarknetErrorCode.TRANSACTION_FAILED

        if isinstance(exception.inner_exc, syscall_utils.HandlerException):
            stark_exception = exception.inner_exc.stark_exception
            code = stark_exception.code
            called_contract_address = exception.inner_exc.called_contract_address
            message_prefix = (
                f"Error in the called contract ({hex(called_contract_address)}):\n"
            )
            # Override python's traceback and keep the Cairo one of the inner exception.
            exception.notes = [message_prefix + str(stark_exception.message)]
        if isinstance(exception.inner_exc, ResourcesError):
            code = StarknetErrorCode.OUT_OF_RESOURCES
        raise StarkException(code=code, message=str(exception)) from exception
    except VmExceptionBase as exception:
        raise StarkException(
            code=StarknetErrorCode.TRANSACTION_FAILED, message=str(exception)
        ) from exception
    except SecurityError as exception:
        raise StarkException(
            code=StarknetErrorCode.SECURITY_ERROR, message=str(exception)
        ) from exception
    except Exception as exception:
        logger.error("Got an unexpected exception.", exc_info=True)
        raise StarkException(
            code=StarknetErrorCode.UNEXPECTED_FAILURE,
            message="Got an unexpected exception during the execution of the transaction.",
        ) from exception

    # Complete handler validations.
    os_utils.validate_and_process_os_context(
        runner=runner,
        syscall_handler=syscall_handler,
        initial_os_context=os_context,
    )

    # When execution starts the stack holds entry_points_args + [ret_fp, ret_pc].
    args_ptr = runner.initial_fp - (len(entry_points_args) + 2)

    # The arguments are touched by the OS and should not be counted as holes, mark them
    # as accessed.
    assert isinstance(args_ptr, RelocatableValue)  # Downcast.
    runner.mark_as_accessed(address=args_ptr, size=len(entry_points_args))

    return runner, syscall_handler


def cairo_rs_py_compute_class_hash_inner(
    contract_class: ContractClass,
    hash_func: Callable[[int, int], int],  # pylint: disable=unused-argument
) -> int:
    program = load_program()
    contract_class_struct = get_contract_class_struct(
        identifiers=program.identifiers, contract_class=contract_class
    )

    runner = cairo_rs_py.CairoRunner(  # pylint: disable=no-member
        program=program.dumps(), entrypoint=None, layout="all", proof_mode=False
    )
    runner.initialize_function_runner()
    hash_ptr = runner.add_additional_hash_builtin()

    run_function_runner(
        runner,
        program,
        "starkware.starknet.core.os.contracts.class_hash",
        hash_ptr=hash_ptr,
        contract_class=contract_class_struct,
        use_full_name=True,
        verify_secure=False,
    )
    _, class_hash = runner.get_return_values(2)
    return class_hash


def run_function_runner(
    runner,
    program,
    func_name: str,
    *args,
    hint_locals: Optional[Dict[str, Any]] = None,
    static_locals: Optional[Dict[str, Any]] = None,
    verify_secure: Optional[bool] = None,
    trace_on_failure: bool = False,
    apply_modulo_to_args: Optional[bool] = None,
    use_full_name: bool = False,
    verify_implicit_args_segment: bool = False,
    **kwargs,
) -> Tuple[Tuple[MaybeRelocatable, ...], Tuple[MaybeRelocatable, ...]]:
    """
    Runs func_name(*args).
    args are converted to Cairo-friendly ones using gen_arg.

    Returns the return values of the function, splitted into 2 tuples of implicit values and
    explicit values. Structs will be flattened to a sequence of felts as part of the returned
    tuple.

    Additional params:
    verify_secure - Run verify_secure_runner to do extra verifications.
    trace_on_failure - Run the tracer in case of failure to help debugging.
    apply_modulo_to_args - Apply modulo operation on integer arguments.
    use_full_name - Treat 'func_name' as a fully qualified identifier name, rather than a
      relative one.
    verify_implicit_args_segment - For each implicit argument, verify that the argument and the
      return value are in the same segment.
    """
    assert isinstance(program, Program)
    entrypoint = program.get_label(func_name, full_name_lookup=use_full_name)

    structs_factory = CairoStructFactory.from_program(program=program)
    func = ScopedName.from_string(scope=func_name)

    full_args_struct = structs_factory.build_func_args(func=func)
    all_args = full_args_struct(*args, **kwargs)  # pylint: disable=not-callable

    try:
        runner.run_from_entrypoint(
            entrypoint,
            all_args,
            typed_args=True,
            hint_locals=hint_locals,
            static_locals=static_locals,
            verify_secure=verify_secure,
            apply_modulo_to_args=apply_modulo_to_args,
        )
    except (VmException, SecurityError, AssertionError) as ex:
        if trace_on_failure:  # Unreachable code
            print(
                f"""\
Got {type(ex).__name__} exception during the execution of {func_name}:
{str(ex)}
"""
            )
            # trace_runner(runner=runner)
        raise

    # The number of implicit arguments is identical to the number of implicit return values.
    n_implicit_ret_vals = structs_factory.get_implicit_args_length(func=func)
    n_explicit_ret_vals = structs_factory.get_explicit_return_values_length(func=func)
    n_ret_vals = n_explicit_ret_vals + n_implicit_ret_vals
    implicit_retvals = tuple(
        runner.get_range(runner.get_ap() - n_ret_vals, n_implicit_ret_vals)
    )

    explicit_retvals = tuple(
        runner.get_range(runner.get_ap() - n_explicit_ret_vals, n_explicit_ret_vals)
    )

    # Verify the memory segments of the implicit arguments.
    if verify_implicit_args_segment:
        implicit_args = all_args[:n_implicit_ret_vals]
        for implicit_arg, implicit_retval in safe_zip(implicit_args, implicit_retvals):
            assert isinstance(
                implicit_arg, RelocatableValue
            ), f"Implicit arguments must be RelocatableValues, {implicit_arg} is not."
            assert isinstance(implicit_retval, RelocatableValue), (
                f"Argument {implicit_arg} is a RelocatableValue, but the returned value "
                f"{implicit_retval} is not."
            )
            assert implicit_arg.segment_index == implicit_retval.segment_index, (
                f"Implicit argument {implicit_arg} is not on the same segment as the returned "
                f"{implicit_retval}."
            )
            assert implicit_retval.offset >= implicit_arg.offset, (
                f"The offset of the returned implicit argument {implicit_retval} is less than "
                f"the offset of the input {implicit_arg}."
            )

    return implicit_retvals, explicit_retvals


def cairo_rs_py_prepare_os_context(
    runner: CairoFunctionRunner,
) -> List[MaybeRelocatable]:
    syscall_segment = runner.add_segment()
    os_context: List[MaybeRelocatable] = [syscall_segment]
    os_context.extend(runner.get_program_builtins_initial_stack())

    return os_context


def cairo_rs_py_validate_and_process_os_context(
    runner: CairoFunctionRunner,
    syscall_handler: syscall_utils.BusinessLogicSysCallHandler,
    initial_os_context: List[MaybeRelocatable],
):
    """
    Validates and processes an OS context that was returned by a transaction.
    Returns the syscall processor object containing the accumulated syscall information.
    """
    os_context_end = runner.get_ap() - 2
    stack_ptr = os_context_end
    # The returned values are os_context, retdata_size, retdata_ptr.
    stack_ptr = runner.get_builtins_final_stack(stack_ptr)

    final_os_context_ptr = stack_ptr - 1
    assert final_os_context_ptr + len(initial_os_context) == os_context_end

    # Validate system calls.
    syscall_base_ptr, syscall_stop_ptr = segment_utils.get_os_segment_ptr_range(
        runner=runner, ptr_offset=SYSCALL_PTR_OFFSET, os_context=initial_os_context
    )

    segment_utils.validate_segment_pointers(
        segments=runner.segments,
        segment_base_ptr=syscall_base_ptr,
        segment_stop_ptr=syscall_stop_ptr,
    )
    syscall_handler.post_run(runner=runner, syscall_stop_ptr=syscall_stop_ptr)


def cairo_rs_py_get_runtime_type(
    cairo_type: CairoType,
) -> Union[Type[int], Type[RelocatableValue]]:
    """
    Given a CairoType returns the expected runtime type.
    """

    if isinstance(cairo_type, TypeFelt):
        return int
    if isinstance(cairo_type, TypePointer) and isinstance(cairo_type.pointee, TypeFelt):
        return RelocatableValue

    raise NotImplementedError(f"Unexpected type: {cairo_type.format()}.")


def handler_exception__str__(self) -> str:
    return self.stark_exception.message


def cairo_rs_py_validate_segment_pointers(
    segments: MemorySegmentManager,
    segment_base_ptr: MaybeRelocatable,
    segment_stop_ptr: MaybeRelocatable,
):
    assert isinstance(segment_base_ptr, RelocatableValue)
    assert (
        segment_base_ptr.offset == 0
    ), f"Segment base pointer must be zero; got {segment_base_ptr.offset}."

    expected_stop_ptr = segment_base_ptr + segments.get_segment_used_size(
        segment_base_ptr.segment_index
    )

    stark_assert(
        expected_stop_ptr == segment_stop_ptr,
        code=StarknetErrorCode.SECURITY_ERROR,
        message=(
            f"Invalid stop pointer for segment. "
            f"Expected: {expected_stop_ptr}, found: {segment_stop_ptr}."
        ),
    )


def cairo_rs_py_monkeypatch():
    setattr(ExecuteEntryPoint, "_run", cairo_rs_py_run)
    setattr(
        sys.modules["starkware.starknet.core.os.class_hash"],
        "class_hash_inner",
        cairo_rs_py_compute_class_hash_inner,
    )
    setattr(
        sys.modules["starkware.starknet.core.os.os_utils"],
        "prepare_os_context",
        cairo_rs_py_prepare_os_context,
    )
    setattr(
        sys.modules["starkware.starknet.core.os.os_utils"],
        "validate_and_process_os_context",
        cairo_rs_py_validate_and_process_os_context,
    )
    setattr(
        sys.modules["starkware.starknet.core.os.segment_utils"],
        "validate_segment_pointers",
        cairo_rs_py_validate_segment_pointers,
    )
    setattr(
        sys.modules["starkware.starknet.core.os.syscall_utils"],
        "get_runtime_type",
        cairo_rs_py_get_runtime_type,
    )
    setattr(syscall_utils.HandlerException, "__str__", handler_exception__str__)
