macro_rules! report_err {
    ($err:ident, $($msg:tt)+) => (
        ::tracing::warn!(error.msg = %$err, error.cause_chain = ?$err, $($msg)+)
    )
}

pub(crate) use report_err;
