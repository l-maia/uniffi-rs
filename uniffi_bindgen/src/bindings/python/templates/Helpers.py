# A handful of classes and functions to support the generated data structures.
# This would be a good candidate for isolating in its own ffi-support lib.

class InternalError(Exception):
    pass

class RustCallStatus(ctypes.Structure):
    """
    Error runtime.
    """
    _fields_ = [
        ("code", ctypes.c_int8),
        ("error_buf", RustBuffer),
    ]

    # These match the values from the uniffi::rustcalls module
    CALL_SUCCESS = 0
    CALL_ERROR = 1
    CALL_PANIC = 2

    def __str__(self):
        if self.code == RustCallStatus.CALL_SUCCESS:
            return "RustCallStatus(CALL_SUCCESS)"
        elif self.code == RustCallStatus.CALL_ERROR:
            return "RustCallStatus(CALL_ERROR)"
        elif self.code == RustCallStatus.CALL_PANIC:
            return "RustCallStatus(CALL_SUCCESS)"
        else:
            return "RustCallStatus(<invalid code>)"

def rust_call(fn, *args):
    # Call a rust function
    return rust_call_with_error(None, fn, *args)

def rust_call_with_error(error_class, fn, *args):
    # Call a rust function and handle any errors
    #
    # This function is used for rust calls that return Result<> and therefore can set the CALL_ERROR status code.
    # error_class must be set to the error class that corresponds to the result.
    call_status = RustCallStatus(code=RustCallStatus.CALL_SUCCESS, error_buf=RustBuffer(0, 0, None))

    args_with_error = args + (ctypes.byref(call_status),)
    result = fn(*args_with_error)
    if call_status.code == RustCallStatus.CALL_SUCCESS:
        return result
    elif call_status.code == RustCallStatus.CALL_ERROR:
        if error_class is None:
            call_status.err_buf.contents.free()
            raise InternalError("rust_call_with_error: CALL_ERROR, but no error class set")
        else:
            raise error_class._lift(call_status.error_buf)
    elif call_status.code == RustCallStatus.CALL_PANIC:
        # When the rust code sees a panic, it tries to construct a RustBuffer
        # with the message.  But if that code panics, then it just sends back
        # an empty buffer.
        if call_status.error_buf.len > 0:
            msg = FfiConverterString._lift(call_status.error_buf)
        else:
            msg = "Unknown rust panic"
        raise InternalError(msg)
    else:
        raise InternalError("Invalid RustCallStatus code: {}".format(
            call_status.code))
