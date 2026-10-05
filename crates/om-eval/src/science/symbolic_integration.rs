//! Exact primitives with retained source domains; definite values require actual interval checks.
use super::integral_rules::Rules;
use super::*;
use om_core::{mul, sqrt};
use om_num::Integer;
fn floor(q: &Rational) -> Integer {
    let d = Integer::from(q.denominator().clone());
    let n = q.numerator() / &d;
    if q.numerator() < &Integer::ZERO && q.numerator() % d != Integer::ZERO {
        n - 1
    } else {
        n
    }
}
fn pi_multiple(e: &Expr) -> Option<Rational> {
    if e.is_zero() {
        return Some(Rational::ZERO);
    }
    if e == &Expr::sym(B::PI) {
        return Some(Rational::ONE);
    }
    if e.is_head(B::PLUS) {
        let mut sum = Rational::ZERO;
        for e in e.args() {
            sum += pi_multiple(e)?;
        }
        return Some(sum);
    }
    if e.is_head(B::TIMES) {
        let mut value = Rational::ONE;
        let mut pi = false;
        for e in e.args() {
            if e == &Expr::sym(B::PI) && !pi {
                pi = true;
            } else {
                value *= e.as_number().and_then(crate::scalar::rational)?;
            }
        }
        return pi.then_some(value);
    }
    None
}
pub(super) fn substitute(
    e: &Expr,
    x: &Expr,
    value: &Expr,
    ctx: &Interrupt,
) -> Result<Expr, EvalError> {
    enum Task<'a> {
        Enter(&'a Expr),
        Build(&'a Expr),
    }
    let mut tasks = vec![Task::Enter(e)];
    let mut values = vec![];
    while let Some(task) = tasks.pop() {
        ctx.tick()?;
        match task {
            Task::Enter(e) if e == x => values.push(value.clone()),
            Task::Enter(e) if e.args().is_empty() => values.push(e.clone()),
            Task::Enter(e) => {
                tasks.push(Task::Build(e));
                tasks.extend(e.args().iter().rev().map(Task::Enter));
            }
            Task::Build(e) => {
                let args = values.split_off(values.len() - e.args().len());
                values.push(if let Some(h) = e.head_symbol() {
                    om_core::func(h, args)
                } else {
                    Expr::normal(e.head(), args)
                });
            }
        }
    }
    values.pop().ok_or_else(|| error("积分端点替换失败"))
}
fn verification_form(e: &Expr, ctx: &Interrupt) -> Result<Expr, EvalError> {
    enum Task<'a> {
        Enter(&'a Expr),
        Build(&'a Expr),
    }
    let mut tasks = vec![Task::Enter(e)];
    let mut values = vec![];
    while let Some(task) = tasks.pop() {
        ctx.tick()?;
        match task {
            Task::Enter(e) if e.args().is_empty() => values.push(e.clone()),
            Task::Enter(e) => {
                tasks.push(Task::Build(e));
                tasks.extend(e.args().iter().rev().map(Task::Enter));
            }
            Task::Build(e) => {
                let args = values.split_off(values.len() - e.args().len());
                let value = if args.len() == 1 {
                    match e.head_symbol() {
                        Some(B::TAN) => {
                            om_core::div(Expr::call(B::SIN, args.clone()), Expr::call(B::COS, args))
                        }
                        Some(B::COT) => {
                            om_core::div(Expr::call(B::COS, args.clone()), Expr::call(B::SIN, args))
                        }
                        Some(B::TANH) => om_core::div(
                            Expr::call(B::SINH, args.clone()),
                            Expr::call(B::COSH, args),
                        ),
                        Some(B::COTH) => om_core::div(
                            Expr::call(B::COSH, args.clone()),
                            Expr::call(B::SINH, args),
                        ),
                        _ => om_core::func(
                            e.head_symbol()
                                .ok_or_else(|| error("不支持复合求导验证头"))?,
                            args,
                        ),
                    }
                } else if let Some(h) = e.head_symbol() {
                    om_core::func(h, args)
                } else {
                    Expr::normal(e.head(), args)
                };
                values.push(value);
            }
        }
    }
    values.pop().ok_or_else(|| error("原函数求导验证失败"))
}
fn no_zero(e: &Expr, x: &Expr, a: &Expr, b: &Expr, ctx: &Interrupt) -> Result<bool, EvalError> {
    if e.is_head(B::TIMES) {
        for e in e.args() {
            ctx.tick()?;
            if !no_zero(e, x, a, b, ctx)? {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    if e.is_head(B::POWER)
        && e.args().len() == 2
        && matches!(e.args()[1].as_number(), Some(Number::Integer(_)))
    {
        return no_zero(&e.args()[0], x, a, b, ctx);
    }
    if matches!(e.head_symbol(), Some(B::SIN | B::COS)) && e.args().len() == 1 {
        if super::integral_poly::affine(&e.args()[0], x, ctx)?.is_none() {
            return Ok(false);
        }
        let (Some(left), Some(right)) = (
            pi_multiple(&substitute(&e.args()[0], x, a, ctx)?),
            pi_multiple(&substitute(&e.args()[0], x, b, ctx)?),
        ) else {
            return Ok(false);
        };
        let (left, right) = if left <= right {
            (left, right)
        } else {
            (right, left)
        };
        let shift = if e.is_head(B::COS) {
            Rational::from_parts(1.into(), 2u32.into())
        } else {
            Rational::ZERO
        };
        return Ok(-floor(&(-(&left - &shift))) > floor(&(right - shift)));
    }
    if e.is_head(B::SINH) && e.args().len() == 1 {
        return no_zero(&e.args()[0], x, a, b, ctx);
    }
    let Some(c) = super::integral_poly::coefficients(e, x, ctx)? else {
        return Ok(false);
    };
    if c.len() > 3 {
        return Ok(false);
    }
    let (Some(a), Some(b), Some(c)) = (
        a.as_number().and_then(crate::scalar::rational),
        b.as_number().and_then(crate::scalar::rational),
        c.iter()
            .map(|e| e.as_number().and_then(crate::scalar::rational))
            .collect::<Option<Vec<_>>>(),
    ) else {
        return Ok(false);
    };
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    let at = |x: &Rational| c.iter().rev().fold(Rational::ZERO, |v, a| v * x + a);
    let left = at(&lo);
    let right = at(&hi);
    if left == Rational::ZERO || right == Rational::ZERO || (&left * &right) < Rational::ZERO {
        return Ok(false);
    }
    if c.len() == 3 && c[2] != Rational::ZERO {
        let middle = -&c[1] / (Rational::from(2) * &c[2]);
        if middle >= lo && middle <= hi && (&left * at(&middle)) <= Rational::ZERO {
            return Ok(false);
        }
    }
    Ok(true)
}
fn real(e: &Expr, ctx: &Interrupt) -> Result<bool, EvalError> {
    Ok(om_simplify::numeval::enclose(e, 128, ctx)?
        .is_some_and(|b| b.im.mid == om_num::BigFloat::ZERO && b.im.rad == om_num::BigFloat::ZERO))
}
fn interval_condition(
    c: &Expr,
    x: &Expr,
    a: &Expr,
    b: &Expr,
    ctx: &Interrupt,
) -> Result<bool, EvalError> {
    if !real(a, ctx)? || !real(b, ctx)? {
        return Ok(false);
    }
    if c.is_head(B::ELEMENT)
        && c.args().len() == 2
        && c.args()[1] == Expr::sym(B::REALS)
        && let Some(p) = super::integral_poly::coefficients(&c.args()[0], x, ctx)?
    {
        for e in p {
            ctx.tick()?;
            if !real(&e, ctx)? {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    if c.is_head(B::GREATER)
        && c.args().len() == 2
        && c.args()[1].is_zero()
        && no_zero(&c.args()[0], x, a, b, ctx)?
    {
        return Ok(
            om_simplify::numeval::enclose(&substitute(&c.args()[0], x, a, ctx)?, 128, ctx)?
                .is_some_and(|v| {
                    v.im.mid == om_num::BigFloat::ZERO
                        && v.im.rad == om_num::BigFloat::ZERO
                        && v.re.mid > v.re.rad
                }),
        );
    }
    Ok(false)
}
pub(super) fn apply(ev: &Evaluator, args: &Args<'_>, ctx: &Interrupt) -> Result<Expr, EvalError> {
    if args.options.keys().any(|key| !matches!(*key, "Mode")) {
        return Err(error("当前exact分支不接受数值算法选项"));
    }
    let axis = args.values[1];
    let (variable, bounds) = if let Some(v) = axis.as_symbol() {
        (v, None)
    } else if axis.is_head(B::LIST) && axis.args().len() == 3 {
        (
            axis.args()[0]
                .as_symbol()
                .ok_or_else(|| error("积分坐标必须是符号"))?,
            Some((&axis.args()[1], &axis.args()[2])),
        )
    } else {
        return Err(error("积分需要变量或[x,a,b]范围"));
    };
    if om_core::builtins::names().contains(&variable.name())
        || om_core::catalog::by_runtime(variable.name()).is_some()
    {
        return Err(error("积分坐标必须是用户符号"));
    }
    let x = Expr::sym(variable);
    let mut fork = ev.fork_readonly();
    let endpoints = bounds
        .map(|(a, b)| Ok::<_, EvalError>((fork.evaluate(a, ctx)?, fork.evaluate(b, ctx)?)))
        .transpose()?;
    let raw = fork.prepare_numeric(args.values[0], &[(variable, None)], ctx)?;
    let mut precision_scan = vec![&raw];
    while let Some(e) = precision_scan.pop() {
        ctx.tick()?;
        if e.as_number().is_some_and(|n| !n.is_exact()) {
            return Err(error(
                "精确原函数验证首版要求精确系数；请显式有理化已有数值，或定积分使用numeric模式",
            ));
        }
        precision_scan.extend(e.args());
    }
    if raw.is_head(B::LIST)
        || raw.is_head(B::RECORD)
        || raw.is_head(B::DATA_TABLE)
        || raw.is_head(B::SERIES_DATA)
    {
        return Err(error("符号积分当前仅接受标量表达式"));
    }
    let mut rules = Rules {
        x: x.clone(),
        ctx,
        conditions: vec![],
        guards: vec![],
    };
    rules.source_guards(&raw)?;
    if endpoints.is_some() {
        let mut pending = vec![&raw];
        while let Some(e) = pending.pop() {
            ctx.tick()?;
            if e.is_head(B::POWER)
                && e.args().len() == 2
                && !e.args()[0].free_of(&x)
                && e.args()[1]
                    .as_number()
                    .and_then(crate::scalar::rational)
                    .is_none()
            {
                return Err(error(
                    "当前exact定积分不能证明符号幂的端点可积与分支条件，保留原式",
                ));
            }
            pending.extend(e.args());
        }
    }
    let expression = om_core::canonicalize(&raw);
    let gaussian = rules.gaussian(&expression)?;
    let primitive = rules
        .integrate(&expression, 0, true)?
        .ok_or_else(|| error("当前符号规则不支持此积分，保留原式；可明确请求numeric"))?;
    fork.scopes.push([(variable, None)].into_iter().collect());
    let derivative = crate::algebra::differentiate(&primitive, &x, ctx)?;
    let residual = om_core::sub(
        verification_form(&derivative, ctx)?,
        verification_form(&expression, ctx)?,
    );
    let verified = fork.evaluate(
        &Expr::call(om_core::Symbol::intern("FullSimplify"), [residual]),
        ctx,
    )?;
    if !verified.is_zero() {
        return Err(error("候选原函数未能通过精确求导验证，保留原式"));
    }
    // Keep coefficient nonzero conditions introduced by the primitive itself.
    let mut coefficients = Rules {
        x: x.clone(),
        ctx,
        conditions: vec![],
        guards: vec![],
    };
    coefficients.source_guards(&primitive)?;
    for guard in coefficients.guards {
        if guard.free_of(&x) {
            rules.nonzero(guard)?;
        }
    }
    let result = if let Some((a, b)) = endpoints {
        let mut remaining = vec![];
        for c in &rules.conditions {
            ctx.tick()?;
            if c.free_of(&x) {
                remaining.push(c.clone());
            } else if !interval_condition(c, &x, &a, &b, ctx)? {
                return Err(error("无法证明局部原函数的定义域条件在整个区间成立"));
            }
        }
        rules.conditions = remaining;
        let positive = |e: &Expr| e.is_head(B::DIRECTED_INFINITY) && e.args() == [Expr::int(1)];
        let negative = |e: &Expr| e.is_head(B::DIRECTED_INFINITY) && e.args() == [Expr::int(-1)];
        if a.is_head(B::DIRECTED_INFINITY) || b.is_head(B::DIRECTED_INFINITY) {
            let Some((coefficient, center, constant)) = gaussian else {
                return Err(error("当前exact无限积分只支持已识别Gaussian"));
            };
            if coefficient
                .as_number()
                .and_then(crate::scalar::rational)
                .is_none_or(|q| q <= Rational::ZERO)
            {
                return Err(error("Gaussian无限积分需明确正实二次系数"));
            }
            let root = sqrt(coefficient);
            let limit = |endpoint: &Expr| -> Result<Expr, EvalError> {
                if positive(endpoint) {
                    Ok(Expr::int(1))
                } else if negative(endpoint) {
                    Ok(Expr::int(-1))
                } else if endpoint.is_head(B::DIRECTED_INFINITY) {
                    Err(error("Gaussian不支持非实方向的无限端点"))
                } else {
                    Ok(Expr::call(
                        om_core::Symbol::intern("Erf"),
                        [mul([
                            root.clone(),
                            om_core::sub(endpoint.clone(), center.clone()),
                        ])],
                    ))
                }
            };
            mul([
                om_core::sub(limit(&b)?, limit(&a)?),
                om_core::div(sqrt(Expr::sym(B::PI)), mul([Expr::int(2), root])),
                Expr::call(B::EXP, [constant]),
            ])
        } else {
            let mut branches = vec![&primitive];
            let mut branch_sensitive = false;
            while let Some(e) = branches.pop() {
                ctx.tick()?;
                let sensitive = (e.is_head(B::LOG) && e.args().len() == 1)
                    || (e.is_head(B::POWER)
                        && e.args().len() == 2
                        && e.args()[1]
                            .as_number()
                            .and_then(crate::scalar::rational)
                            .is_none_or(|q| q.denominator() != &1u32.into()));
                branch_sensitive |= sensitive;
                if sensitive && !e.args()[0].free_of(&x) && !no_zero(&e.args()[0], &x, &a, &b, ctx)?
                {
                    return Err(error("当前exact不能证明原函数分支在整个区间连续，保留原式"));
                }
                branches.extend(e.args());
            }
            if branch_sensitive {
                for endpoint in [&a, &b] {
                    if om_simplify::numeval::enclose(endpoint, 128, ctx)?.is_none_or(|b| {
                        b.im.mid != om_num::BigFloat::ZERO || b.im.rad != om_num::BigFloat::ZERO
                    }) {
                        return Err(error(
                            "分支敏感的exact定积分首版需要可证明的实端点，不猜测复路径",
                        ));
                    }
                }
            }
            let rational_view = om_simplify::convert::to_rational_function_with(
                &expression,
                std::slice::from_ref(&x),
                ctx,
            )?;
            let mut actual_poles = None;
            if rational_view
                .as_ref()
                .is_some_and(|v| v.gens == [x.clone()])
                && let Some(cancelled) = crate::algebra::partial_fractions(&expression, &x, ctx)?
            {
                let mut actual = Rules {
                    x: x.clone(),
                    ctx,
                    conditions: vec![],
                    guards: vec![],
                };
                actual.source_guards(&cancelled)?;
                actual_poles = Some(actual.guards);
            }
            for guard in actual_poles.as_ref().unwrap_or(&rules.guards) {
                if !guard.free_of(&x) && !no_zero(guard, &x, &a, &b, ctx)? {
                    return Err(error(
                        "无法证明积分区间无原式奇点/分支障碍，保留exact；不使用端点相减伪造成功",
                    ));
                }
            }
            om_core::sub(
                substitute(&primitive, &x, &b, ctx)?,
                substitute(&primitive, &x, &a, ctx)?,
            )
        }
    } else {
        if !rules.guards.is_empty() {
            rules
                .conditions
                .push(Expr::call(B::ELEMENT, [x.clone(), Expr::sym(B::REALS)]));
        }
        let mut branch_scan = vec![&primitive];
        while let Some(e) = branch_scan.pop() {
            ctx.tick()?;
            let branch_sensitive = (e.is_head(B::LOG) && e.args().len() == 1)
                || (e.is_head(B::POWER)
                    && e.args().len() == 2
                    && e.args()[1]
                        .as_number()
                        .and_then(crate::scalar::rational)
                        .is_none_or(|q| q.denominator() != &1u32.into()));
            let argument = branch_sensitive.then(|| &e.args()[0]);
            if let Some(argument) = argument
                && !argument.free_of(&x)
            {
                rules.conditions.push(Expr::call(
                    B::OR,
                    [
                        Expr::call(
                            B::UNEQUAL,
                            [Expr::call(B::IM, [argument.clone()]), Expr::int(0)],
                        ),
                        Expr::call(
                            B::GREATER,
                            [Expr::call(B::RE, [argument.clone()]), Expr::int(0)],
                        ),
                    ],
                ));
            }
            branch_scan.extend(e.args());
        }
        for guard in rules.guards.clone() {
            if !guard.free_of(&x) {
                rules.nonzero(guard)?;
            }
        }
        primitive
    };
    if rules.conditions.is_empty() {
        fork.evaluate(&result, ctx)
    } else {
        fork.evaluate(
            &Expr::call(
                B::CONDITIONAL_EXPRESSION,
                [result, Expr::call(B::AND, rules.conditions)],
            ),
            ctx,
        )
    }
}
