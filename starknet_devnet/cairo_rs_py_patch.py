"""Patch starknet methods to use cairo_rs_py"""

# pylint: disable=bare-except
# pylint: disable=missing-function-docstring
# pylint: disable=protected-access
# pylint: disable=too-many-locals
# pylint: disable=no-name-in-module
# pylint: disable=too-many-arguments

import logging
import sys
from typing import Any, Callable, Dict, List, Optional, Tuple, Type, Union, cast

from cairo_rs_py import CairoRunner, RelocatableValue
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
from starkware.cairo.lang.vm.utils import ResourcesError, RunResources
from starkware.cairo.lang.vm.vm_exceptions import (
    SecurityError,
    VmException,
    VmExceptionBase,
)
from starkware.python.utils import as_non_optional, safe_zip
from starkware.starknet.business_logic.execution.execute_entry_point import (
    ExecuteEntryPoint,
    ExecutionResourcesManager,
)
from starkware.starknet.business_logic.execution.objects import (
    CallInfo,
    TransactionExecutionContext,
)
from starkware.starknet.business_logic.state.state_api import SyncState
from starkware.starknet.business_logic.utils import (
    get_call_result,
    get_call_result_for_version0_class,
)
from starkware.starknet.core.os import os_utils, syscall_utils
from starkware.starknet.core.os.contract_class.class_hash import (
    get_contract_class_struct,
    load_contract_class_cairo_program,
)
from starkware.starknet.core.os.contract_class.compiled_class_hash_utils import (
    get_compiled_class_struct,
    load_compiled_class_cairo_program,
)
from starkware.starknet.core.os.contract_class.deprecated_class_hash import (
    get_deprecated_contract_class_struct,
    load_program,
)
from starkware.starknet.core.os.syscall_handler import (
    BusinessLogicSyscallHandler,
    DeprecatedBlSyscallHandler,
    SyscallHandlerBase,
)
from starkware.starknet.definitions.error_codes import StarknetErrorCode
from starkware.starknet.definitions.general_config import StarknetGeneralConfig
from starkware.starknet.public import abi as starknet_abi
from starkware.starknet.services.api.contract_class.contract_class import (
    CompiledClass,
    ContractClass,
    DeprecatedCompiledClass,
)
from starkware.starkware_utils.error_handling import (
    ErrorCode,
    StarkException,
    stark_assert,
    wrap_with_stark_exception,
)

logger = logging.getLogger(__name__)


def cairo_rs_py_execute_version0_class(
    self,
    state: SyncState,
    resources_manager: ExecutionResourcesManager,
    tx_execution_context: TransactionExecutionContext,
    class_hash: int,
    compiled_class: DeprecatedCompiledClass,
    general_config: StarknetGeneralConfig,
) -> CallInfo:
    # Fix the current resources usage, in order to calculate the usage of this run at the end.
    previous_cairo_usage = resources_manager.cairo_usage

    # Prepare runner.
    with wrap_with_stark_exception(code=StarknetErrorCode.SECURITY_ERROR):
        runner = CairoRunner(program=compiled_class.program.dumps(), entrypoint=None)
        runner.initialize_function_runner(add_segment_arena_builtin=False)

    # Prepare implicit arguments.
    implicit_args = os_utils.prepare_os_implicit_args_for_version0_class(runner=runner)

    # Prepare syscall handler.
    initial_syscall_ptr = cast(
        RelocatableValue, implicit_args[starknet_abi.SYSCALL_PTR_OFFSET_IN_VERSION0]
    )
    syscall_handler = DeprecatedBlSyscallHandler(
        execute_entry_point_cls=ExecuteEntryPoint,
        tx_execution_context=tx_execution_context,
        state=state,
        resources_manager=resources_manager,
        caller_address=self.caller_address,
        contract_address=self.contract_address,
        general_config=general_config,
        initial_syscall_ptr=initial_syscall_ptr,
        segments=runner.segments,
    )

    # Prepare all arguments.
    entry_point_args = [
        self.entry_point_selector,
        implicit_args,
        len(self.calldata),
        # Allocate and mark the segment as read-only (to mark every input array as read-only).
        syscall_handler._allocate_segment(data=self.calldata),
    ]

    # Get offset to run from.
    entry_point = self._get_selected_entry_point(
        compiled_class=compiled_class, class_hash=class_hash
    )
    entry_point_offset = entry_point.offset

    # Run.
    self._run(
        runner=runner,
        entry_point_offset=entry_point_offset,
        entry_point_args=entry_point_args,
        hint_locals={"syscall_handler": syscall_handler},
        run_resources=tx_execution_context.run_resources,
    )

    # Complete validations.
    os_utils.validate_and_process_os_context_for_version0_class(
        runner=runner,
        syscall_handler=syscall_handler,
        initial_os_context=implicit_args,
    )

    # Update resources usage (for the bouncer and fee calculation).
    resources_manager.cairo_usage += runner.get_execution_resources()

    # Build and return the call info.
    return self._build_call_info(
        storage=syscall_handler.starknet_storage,
        events=syscall_handler.events,
        l2_to_l1_messages=syscall_handler.l2_to_l1_messages,
        internal_calls=syscall_handler.internal_calls,
        execution_resources=resources_manager.cairo_usage - previous_cairo_usage,
        result=get_call_result_for_version0_class(runner=runner),
        class_hash=class_hash,
    )


def cairo_rs_py_execute(
    self,
    state: SyncState,
    compiled_class: CompiledClass,
    class_hash: int,
    resources_manager: ExecutionResourcesManager,
    general_config: StarknetGeneralConfig,
    tx_execution_context: TransactionExecutionContext,
    support_reverted: bool,
) -> CallInfo:
    # Fix the current resources usage, in order to calculate the usage of this run at the end.
    previous_cairo_usage = resources_manager.cairo_usage

    # Prepare runner.
    entry_point = self._get_selected_entry_point(
        compiled_class=compiled_class, class_hash=class_hash
    )
    program = compiled_class.get_runnable_program(
        entrypoint_builtins=as_non_optional(entry_point.builtins)
    )
    with wrap_with_stark_exception(code=StarknetErrorCode.SECURITY_ERROR):
        runner = CairoRunner(  # pylint: disable=no-member
            program=program.dumps(), entrypoint=None
        )
    runner.initialize_function_runner(add_segment_arena_builtin=True)

    # Prepare implicit arguments.
    implicit_args = os_utils.prepare_os_implicit_args(
        runner=runner, gas=self.initial_gas
    )

    # Prepare syscall handler.
    initial_syscall_ptr = cast(RelocatableValue, implicit_args[-1])
    syscall_handler = BusinessLogicSyscallHandler(
        state=state,
        resources_manager=resources_manager,
        segments=runner.segments,
        tx_execution_context=tx_execution_context,
        initial_syscall_ptr=initial_syscall_ptr,
        general_config=general_config,
        entry_point=self,
        support_reverted=support_reverted,
    )

    # Load the builtin costs; Cairo 1.0 programs are expected to end with a `ret` opcode
    # followed by a pointer to the builtin costs.
    core_program_end_ptr = runner.program_base + len(program.data)
    builtin_costs = [0, 0, 0, 0, 0]
    # Use allocate_segment to mark it as read-only.
    builtin_cost_ptr = syscall_handler.allocate_segment(data=builtin_costs)
    program_extra_data: List[MaybeRelocatable] = [0x208B7FFF7FFF7FFE, builtin_cost_ptr]
    runner.load_data(ptr=core_program_end_ptr, data=program_extra_data)

    # Arrange all arguments.

    # Allocate and mark the segment as read-only (to mark every input array as read-only).
    calldata_start = syscall_handler.allocate_segment(data=self.calldata)
    calldata_end = calldata_start + len(self.calldata)
    entry_point_args = [
        # Note that unlike old classes, implicit arguments appear flat in the stack.
        *implicit_args,
        calldata_start,
        calldata_end,
    ]
    # Run.
    self._run(
        runner=runner,
        entry_point_offset=entry_point.offset,
        entry_point_args=entry_point_args,
        hint_locals={"syscall_handler": syscall_handler},
        run_resources=tx_execution_context.run_resources,
        program_segment_size=len(program.data) + len(program_extra_data),
    )

    # We should not count (possibly) unsued code as holes.
    runner.mark_as_accessed(address=core_program_end_ptr, size=len(program_extra_data))

    # Complete validations.
    os_utils.validate_and_process_os_implicit_args(
        runner=runner,
        syscall_handler=syscall_handler,
        initial_implicit_args=implicit_args,
    )

    # Update resources usage (for the bouncer and fee calculation).
    resources_manager.cairo_usage += runner.get_execution_resources()

    # Build and return the call info.
    return self._build_call_info(
        class_hash=class_hash,
        execution_resources=resources_manager.cairo_usage - previous_cairo_usage,
        storage=syscall_handler.storage,
        result=get_call_result(runner=runner, initial_gas=self.initial_gas),
        events=syscall_handler.events,
        l2_to_l1_messages=syscall_handler.l2_to_l1_messages,
        internal_calls=syscall_handler.internal_calls,
    )


def cairo_rs_py_run(
    # pylint: disable=unused-argument
    self,
    runner: CairoRunner,
    entry_point_offset: int,
    entry_point_args,
    hint_locals: Dict[str, Any],
    run_resources: RunResources,
    program_segment_size: Optional[int] = None,
):
    """
    Runs the runner from the entrypoint offset with the given arguments.

    Wraps VM exceptions with StarkException.
    """
    try:
        runner.run_from_entrypoint(
            entry_point_offset,
            args=entry_point_args,
            hint_locals=hint_locals,
            static_locals={
                "__find_element_max_size": 2**20,
                "__squash_dict_max_size": 2**20,
                "__keccak_max_size": 2**20,
                "__usort_max_size": 2**20,
                "__chained_ec_op_max_len": 1000,
            },
            run_resources=run_resources,
            verify_secure=True,
            program_segment_size=program_segment_size,
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

    # When execution starts the stack holds entry_points_args + [ret_fp, ret_pc].
    args_ptr = runner.initial_fp - (len(entry_point_args) + 2)

    # The arguments are touched by the OS and should not be counted as holes, mark them
    # as accessed.
    assert isinstance(args_ptr, RelocatableValue)  # Downcast.
    runner.mark_as_accessed(address=args_ptr, size=len(entry_point_args))


def cairo_rs_py_compute_class_hash_inner(
    contract_class: ContractClass,
) -> int:
    program = load_contract_class_cairo_program()
    contract_class_struct = get_contract_class_struct(
        identifiers=program.identifiers, contract_class=contract_class
    )

    runner = CairoRunner(  # pylint: disable=no-member
        program=program.dumps(), entrypoint=None
    )
    runner.initialize_function_runner(add_segment_arena_builtin=False)
    poseidon_ptr = runner.get_poseidon_builtin_base()
    range_check_ptr = runner.get_range_check_builtin_base()

    run_function_runner(
        runner,
        program,
        "starkware.starknet.core.os.contract_class.contract_class.class_hash",
        poseidon_ptr=poseidon_ptr,
        range_check_ptr=range_check_ptr,
        contract_class=contract_class_struct,
        use_full_name=True,
        verify_secure=False,
    )
    _, class_hash = runner.get_return_values(2)
    return class_hash


def cairo_rs_py_compute_compiled_class_hash_inner(compiled_class: CompiledClass) -> int:
    program = load_compiled_class_cairo_program()
    compiled_class_struct = get_compiled_class_struct(
        identifiers=program.identifiers, compiled_class=compiled_class
    )
    runner = CairoRunner(  # pylint: disable=no-member
        program=program.dumps(), entrypoint=None
    )
    runner.initialize_function_runner(add_segment_arena_builtin=False)
    poseidon_ptr = runner.get_poseidon_builtin_base()

    run_function_runner(
        runner,
        program,
        "starkware.starknet.core.os.contract_class.compiled_class.compiled_class_hash",
        poseidon_ptr=poseidon_ptr,
        compiled_class=compiled_class_struct,
        use_full_name=True,
        verify_secure=False,
    )
    _, class_hash = runner.get_return_values(2)
    return class_hash


def cairo_rs_py_compute_deprecated_class_hash_inner(
    # pylint: disable=unused-argument
    contract_class: DeprecatedCompiledClass,
    hash_func: Callable[[int, int], int],
) -> int:
    program = load_program()
    compiled_class_struct = get_deprecated_contract_class_struct(
        identifiers=program.identifiers, contract_class=contract_class
    )
    runner = CairoRunner(  # pylint: disable=no-member
        program=program.dumps(), entrypoint=None
    )
    runner.initialize_function_runner(add_segment_arena_builtin=False)
    hash_ptr = runner.get_hash_builtin_base()

    run_function_runner(
        runner,
        program,
        "starkware.starknet.core.os.contract_class.deprecated_compiled_class."
        + "deprecated_compiled_class_hash",
        hash_ptr=hash_ptr,
        compiled_class=compiled_class_struct,
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
            args=all_args,
            hint_locals=hint_locals,
            static_locals=static_locals,
            typed_args=True,
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


def cairo_rs_py_prepare_builtins(runner: CairoRunner) -> List[MaybeRelocatable]:
    """
    Initializes and returns the builtin segments.
    """
    return runner.get_program_builtins_initial_stack()


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
        segment_index=segment_base_ptr.segment_index
    )

    stark_assert(
        expected_stop_ptr == segment_stop_ptr,
        code=StarknetErrorCode.SECURITY_ERROR,
        message=(
            f"Invalid stop pointer for segment. "
            f"Expected: {expected_stop_ptr}, found: {segment_stop_ptr}."
        ),
    )


def cairo_rs_py_validate_builtins(
    runner: CairoRunner, builtins_end: MaybeRelocatable, n_builtins: int
):
    stack_ptr = builtins_end
    with wrap_with_stark_exception(code=StarknetErrorCode.SECURITY_ERROR):
        stack_ptr = runner.get_builtins_final_stack(stack_ptr)
    builtins_start = stack_ptr
    assert builtins_start + n_builtins == builtins_end, "Bad returned builtins."


def cairo_rs_py_validate_read_only_segments(self, runner: CairoRunner):
    """
    Validates that there were no out of bounds writes to read-only segments and marks
    them as accessed.
    """
    for segment_ptr, segment_size in self.read_only_segments:
        # Check segment usage individually instead of comparing the whole MemorySegmentManager structures
        assert segment_size == runner.segments.get_segment_size(
            segment_ptr.segment_index
        ), "Inconsistent segments."
        used_size = self.segments.get_segment_used_size(
            segment_index=segment_ptr.segment_index
        )
        assert used_size == runner.segments.get_segment_used_size(
            segment_ptr.segment_index
        ), "Inconsistent segments."
        stark_assert(
            used_size == segment_size,
            code=StarknetErrorCode.SECURITY_ERROR,
            message="Out of bounds write to a read-only segment.",
        )
        runner.mark_as_accessed(address=segment_ptr, size=segment_size)


def cairo_rs_py_get_felt_range(self, start_addr: Any, end_addr: Any) -> List[int]:
    assert isinstance(start_addr, RelocatableValue)
    assert isinstance(end_addr, RelocatableValue)
    assert start_addr.segment_index == end_addr.segment_index, (
        "Inconsistent start and end segment indices "
        f"({start_addr.segment_index} != {end_addr.segment_index})."
    )
    assert start_addr.offset <= end_addr.offset, (
        "The start offset cannot be greater than the end offset"
        f"({start_addr.offset} > {end_addr.offset})."
    )
    size = end_addr.offset - start_addr.offset
    return self.segments.memory.get_range_as_ints(addr=start_addr, size=size)


def cairo_rs_py_monkeypatch():
    setattr(ExecuteEntryPoint, "_execute", cairo_rs_py_execute)
    setattr(ExecuteEntryPoint, "_run", cairo_rs_py_run)
    setattr(
        BusinessLogicSyscallHandler,
        "_validate_read_only_segments",
        cairo_rs_py_validate_read_only_segments,
    )
    setattr(
        DeprecatedBlSyscallHandler,
        "validate_read_only_segments",
        cairo_rs_py_validate_read_only_segments,
    )
    setattr(
        ExecuteEntryPoint, "_execute_version0_class", cairo_rs_py_execute_version0_class
    )
    setattr(
        sys.modules["starkware.starknet.core.os.contract_class.class_hash"],
        "_compute_class_hash_inner",
        cairo_rs_py_compute_class_hash_inner,
    )
    setattr(
        sys.modules["starkware.starknet.core.os.contract_class.compiled_class_hash"],
        "_compute_compiled_class_hash_inner",
        cairo_rs_py_compute_compiled_class_hash_inner,
    )
    setattr(
        sys.modules["starkware.starknet.core.os.contract_class.deprecated_class_hash"],
        "compute_deprecated_class_hash_inner",
        cairo_rs_py_compute_deprecated_class_hash_inner,
    )
    setattr(
        sys.modules["starkware.starknet.core.os.os_utils"],
        "prepare_builtins",
        cairo_rs_py_prepare_builtins,
    )
    setattr(
        sys.modules["starkware.starknet.core.os.os_utils"],
        "validate_builtins",
        cairo_rs_py_validate_builtins,
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
    setattr(SyscallHandlerBase, "_get_felt_range", cairo_rs_py_get_felt_range)
