use crate::{
    assist_context::{AssistContext, Assists},
    AssistId, AssistKind,
};

use syntax::{
    ast::{IndexExpr, BlockExpr, MethodCallExpr, ExprStmt, CallExpr, edit_in_place::Indent, LetStmt, BinExpr},
    SyntaxKind::{STMT_LIST, EXPR_STMT, INDEX_EXPR, LET_STMT, PATH_EXPR, BIN_EXPR}, 
    TextSize, Direction
};
use itertools::Itertools;
use stdx::format_to;
use syntax::{
    ast::{
        self,
        AstNode,
        HasArgList,
    },
    SyntaxNode, TextRange, T,
};

// Assist: convert_unsafe_to_safe
//
// Replace unsafe code with safe version.
//
// ```
// fn main() {
//
//     let mut buffer = Vec::with_capacity(cap);
//
//     unsafe {
//         buffer.set_len(cap); 
//         foo();
//     }
// }
// ```
// ->
// ```
// fn main() {
//    
//     let mut buffer = vec![0; cap];
//  
//     unsafe {
//         foo();
//     }
//     
// }
// ```

pub enum UnsafePattern {
    SetVecCapacity,
    ReserveVec,
    WriteVec,
    UnitializedVec,
    CopyWithin,
    GetUncheck,
    GetUncheckMut,
    CopyNonOverlap,
    CStringFromVec,
    CStringLength,
    BytesToUTFString,
    TransmuteTo,
}

impl std::fmt::Display for UnsafePattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnsafePattern::SetVecCapacity => write!(f, "with_capacity"),
            UnsafePattern::ReserveVec => write!(f, "reserve"),
            UnsafePattern::WriteVec => write!(f, "write"),
            UnsafePattern::UnitializedVec => write!(f, "set_len"),
            UnsafePattern::CopyWithin => write!(f, "ptr::copy"),
            UnsafePattern::GetUncheck => write!(f, "get_unchecked"),
            UnsafePattern::GetUncheckMut => write!(f, "get_unchecked_mut"),
            UnsafePattern::CopyNonOverlap => write!(f, "ptr::copy_nonoverlapping"),
            UnsafePattern::CStringFromVec => write!(f, "CString::from_vec_unchecked"),
            UnsafePattern::CStringLength => write!(f, "libc::strlen"),
            UnsafePattern::BytesToUTFString => write!(f, "str::from_utf8_unchecked"),
            UnsafePattern::TransmuteTo => write!(f, "mem::transmute"),
        }
    }
}

enum TargetTypes {
    String,
}

impl std::fmt::Display for TargetTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetTypes::String => write!(f, "&str"),
        }
    }
}

pub fn generate_safevec_format(mcall: &MethodCallExpr) -> Option<String> {

    // Obtain the variable Expr that presents the buffer/vector
    let receiver = mcall.receiver()?;

    let closure_body = mcall.arg_list()?.args().exactly_one().ok()?;

    let mut buf = String::new();

    format_to!(buf, "let mut {} = vec![0; {}];", receiver, closure_body);

    buf.push('\n');

    return Some(buf);

}

pub fn generate_resizevec_format(mcall: &MethodCallExpr) -> Option<String> {

    // Obtain the variable Expr that presents the buffer/vector
    let receiver = mcall.receiver()?;

    let closure_body = mcall.arg_list()?.args().exactly_one().ok()?;

    let mut buf = String::new();

    format_to!(buf, "{}.resize({}, 0);", receiver, closure_body);

    buf.push('\n');

    return Some(buf);

}

fn check_single_expr(target_expr: &ExprStmt) -> bool {

    // Check if the unsafe bloack only contains one expr
    if target_expr.syntax().prev_sibling().is_none() && target_expr.syntax().next_sibling().is_none() {
        return true;
    }
    return false;
}

fn delet_replace_source_code(acc: &mut Assists, let_target: TextRange, target_range: TextRange, buf: &String) {

    acc.add(
        AssistId("convert_unsafe_to_safe", AssistKind::RefactorRewrite),
        "Convert Unsafe to Safe",
        target_range,
        |edit| {
            edit.delete(target_range);
            edit.replace(let_target, buf)
        },
    );
}

fn convert_to_auto_vec_initialization(acc: &mut Assists, target_expr: &SyntaxNode, unsafe_range: TextRange, unsafe_expr: &BlockExpr) -> Option<()> {

    let mcall = target_expr.parent().and_then(ast::MethodCallExpr::cast)?;

    let buf = if let Some(buffer) = generate_safevec_format(&mcall) {buffer} else { return None; };

    let buf_resize = if let Some(buffer) = generate_resizevec_format(&mcall) {buffer} else { return None; };

    let mut target_range = unsafe_range;

    if mcall.syntax().parent()?.kind() == EXPR_STMT {
        // Declare the target text range for modification.
        let target_expr = mcall.syntax().parent().and_then(ast::ExprStmt::cast)?;

        target_range = target_expr.syntax().text_range();
        if check_single_expr(&target_expr) {
            target_range = unsafe_range;
        }
    }

    let mut backward_list = unsafe_expr.syntax().siblings(Direction::Prev);

    if unsafe_expr.syntax().parent()?.kind() != STMT_LIST {
        backward_list = unsafe_expr.syntax().parent()?.siblings(Direction::Prev);
    }

    // for iter in unsafe_expr.syntax().parent()?.siblings(Direction::Prev) {
    for iter in backward_list {

        if iter.to_string().contains(&UnsafePattern::SetVecCapacity.to_string()) && iter.to_string().contains(&mcall.receiver()?.to_string()) {

            let let_expr = ast::LetStmt::cast(iter)?;

            let let_target = let_expr.syntax().text_range();
            // Delete the "set_len" expression in unsafe code block and insert the auto initialized vec/buf
            delet_replace_source_code(acc, let_target, target_range, &buf);

            return None;

        }

        if iter.to_string().contains(&UnsafePattern::ReserveVec.to_string()) && iter.to_string().contains(&mcall.receiver()?.to_string()) {

            let expr_stmt = ast::ExprStmt::cast(iter)?;

            let expr_target = expr_stmt.syntax().text_range();
            // Delete the "set_len" expression in unsafe code block and insert the auto initialized vec/buf
            delet_replace_source_code(acc, expr_target, target_range, &buf_resize);

            return None;
        }
    }
    return None;
}

pub fn generate_copywithin_string(base_expr: String, start_pos: String, end_pos: String, count_expr: String) -> String {

    let mut buf = String::new();

    format_to!(buf, "{}.copy_within({}..{}, {});", base_expr, start_pos, count_expr, end_pos);

    buf.push('\n');

    return buf;

}

struct CpyWithinInfo {
    base_expr: String,
    start_pos: String,
    end_pos: String,
    count_expr: String,
}

fn collect_cpy_within_info(mcall: &CallExpr, src_expr: IndexExpr, dst_expr: IndexExpr) -> Option<CpyWithinInfo> {

    let count_expr = mcall.arg_list()?.args().nth(2)?.to_string();

    let base_expr = src_expr.base()?.to_string();

    let start_pos = src_expr.index()?.to_string().trim_matches('.').to_string();

    let end_pos = dst_expr.index()?.to_string().trim_matches('.').to_string();

    return Some(CpyWithinInfo {base_expr, start_pos, end_pos, count_expr});
}

fn delet_insert_source_code(acc: &mut Assists, target_range: TextRange, position: TextSize, new_buf: &String) {

    acc.add(
        AssistId("convert_unsafe_to_safe", AssistKind::RefactorRewrite),
        "Convert Unsafe to Safe",
        target_range,
        |edit| {
            edit.delete(target_range);
            edit.insert(position + TextSize::of('\n'), new_buf)
        },
    );
}

fn collect_ptrcpy_path_info(mcall: &CallExpr, index: usize, unsafe_expr: &BlockExpr) -> Option<IndexExpr> {
    
    let src_expr;

    let mut backward_list = unsafe_expr.syntax().siblings(Direction::Prev);

    if unsafe_expr.syntax().parent()?.kind() != STMT_LIST {
        backward_list = unsafe_expr.syntax().parent()?.siblings(Direction::Prev);
    }

    for backward_slice in backward_list {
        if backward_slice.to_string().contains(&mcall.arg_list()?.args().nth(index)?.to_string()) && backward_slice.kind() == LET_STMT {
            let src_def = ast::LetStmt::cast(backward_slice)?;
            if src_def.syntax().last_child()?.children().nth(0)?.kind() == INDEX_EXPR {
                src_expr = ast::IndexExpr::cast(src_def.syntax().last_child()?.children().nth(0)?)?;
                return Some(src_expr);
            } else {
                src_expr = ast::IndexExpr::cast(src_def.syntax().last_child()?.children().nth(0)?.children().nth(0)?)?;
                return Some(src_expr);
            }
        }
    }
    None
}

fn collect_ptrcpy_expr_info(mcall: &CallExpr, index: usize) -> Option<IndexExpr> {
    let src_expr;

    if mcall.arg_list()?.args().nth(index)?.syntax().children().nth(0)?.kind() == INDEX_EXPR {
        src_expr = ast::IndexExpr::cast(mcall.arg_list()?.args().nth(index)?.syntax().children().nth(0)?)?;
    } else {
        src_expr = ast::IndexExpr::cast(mcall.arg_list()?.args().nth(index)?.syntax().children().nth(0)?.children().nth(0)?)?;
    }
    return Some(src_expr);
}

pub fn generate_copywithin_format(mcall: &CallExpr, unsafe_expr: &BlockExpr) -> Option<String> {

    let src_expr;
    if mcall.arg_list()?.args().nth(0)?.syntax().kind() == PATH_EXPR {
        src_expr = collect_ptrcpy_path_info(&mcall, 0, &unsafe_expr)?;
    } else {
        src_expr = collect_ptrcpy_expr_info(&mcall, 0)?;
    }

    let dst_expr;
    if mcall.arg_list()?.args().nth(1)?.syntax().kind() == PATH_EXPR {
        dst_expr = collect_ptrcpy_path_info(&mcall, 1, &unsafe_expr)?;
    } else {
        dst_expr = collect_ptrcpy_expr_info(&mcall, 1)?;
    }

    let CpyWithinInfo { base_expr, start_pos, end_pos, count_expr} = collect_cpy_within_info(&mcall, src_expr, dst_expr)?;

    let buf = generate_copywithin_string(base_expr, start_pos, end_pos, count_expr);

    return Some(buf);

}

fn replace_source_code(acc: &mut Assists, target_range: TextRange, buf: &String) {
    acc.add(
        AssistId("convert_unsafe_to_safe", AssistKind::RefactorRewrite),
        "Convert Unsafe to Safe",
        target_range,
        |edit| {
            edit.replace(target_range, buf)
        },
    );
}

fn reindent_expr(unsafe_expr: &BlockExpr, acc: &mut Assists, target_range: TextRange, buf: &String) -> Option<()> {

    let position;

    if unsafe_expr.syntax().prev_sibling().is_none() {
        position = unsafe_expr.syntax().parent()?.prev_sibling()?.text_range().end();
    } else {
        position = unsafe_expr.syntax().prev_sibling()?.text_range().end();
    }

    let indent_level = unsafe_expr.indent_level();

    let mut new_buf = String::new();

    format_to!(new_buf, "{}{}", indent_level, buf);

    new_buf.push('\n');

    delet_insert_source_code(acc, target_range, position, &new_buf);

    return None;

}

fn convert_to_copy_within(acc: &mut Assists, target_expr: &SyntaxNode, unsafe_range: TextRange, unsafe_expr: &BlockExpr) -> Option<()> {

    let mcall = target_expr.parent().and_then(ast::CallExpr::cast)?;

    let target_expr = mcall.syntax().parent().and_then(ast::ExprStmt::cast)?;

    let mut target_range = target_expr.syntax().text_range();

    let buf = generate_copywithin_format(&mcall, &unsafe_expr)?;

    if check_single_expr(&target_expr) {
        target_range = unsafe_range;
        replace_source_code(acc, target_range, &buf);
        return None;
    }

    return reindent_expr(unsafe_expr, acc, target_range, &buf);
}

pub fn generate_let_get_mut(mcall: &MethodCallExpr, let_expr: &LetStmt) -> Option<String> {

    // Obtain the variable Expr that presents the buffer/vector
    let receiver = mcall.receiver()?;

    let closure_body = mcall.arg_list()?.args().exactly_one().ok()?;

    let pat = let_expr.pat()?;

    let mut buf = String::new();

    if let_expr.initializer()?.to_string().contains("mut") {
        format_to!(buf, "let {} = {}.get_mut({}).unwrap();", pat, receiver, closure_body);
    } else {
        format_to!(buf, "let {} = {}.get({}).unwrap();", pat, receiver, closure_body);
    }

    buf.push('\n');

    return Some(buf);
}

pub fn generate_get_mut(mcall: &MethodCallExpr, expr: &BinExpr) -> Option<String> {

    // Obtain the variable Expr that presents the buffer/vector
    let receiver = mcall.receiver()?;

    let closure_body = mcall.arg_list()?.args().exactly_one().ok()?;

    let pat = expr.lhs()?;

    let mut buf = String::new();

    if expr.rhs()?.to_string().contains("mut") {
        format_to!(buf, "{} = {}.get_mut({}).unwrap();", pat, receiver, closure_body);
    } else {
        format_to!(buf, "{} = {}.get({}).unwrap();", pat, receiver, closure_body);
    }

    buf.push('\n');

    return Some(buf);
}

fn check_single_let_expr(target_expr: &LetStmt) -> bool {

    // Check if the unsafe bloack only contains one expr
    if target_expr.syntax().prev_sibling().is_none() && target_expr.syntax().next_sibling().is_none() {
        return true;
    }
    return false;
}

fn convert_to_get_mut(acc: &mut Assists, target_expr: &SyntaxNode, unsafe_range: TextRange, unsafe_expr: &BlockExpr) -> Option<()> {

    let mcall = target_expr.parent().and_then(ast::MethodCallExpr::cast)?;

    if mcall.syntax().parent()?.kind() == BIN_EXPR {
        let target_expr = mcall.syntax().parent().and_then(ast::BinExpr::cast)?;

        let mut target_range = target_expr.syntax().parent()?.text_range();

        let buf = generate_get_mut(&mcall, &target_expr)?;
        
        if check_single_bin_expr(&target_expr)? == true {
            target_range = unsafe_range;
            replace_source_code(acc, target_range, &buf);
            return None;
        }
        return reindent_expr(unsafe_expr, acc, target_range, &buf);
    }

    let let_expr = mcall.syntax().parent().and_then(ast::LetStmt::cast)?;

    let buf = generate_let_get_mut(&mcall, &let_expr)?;

    let mut target_range = let_expr.syntax().text_range();
    if check_single_let_expr(&let_expr) {
        target_range = unsafe_range;
        replace_source_code(acc, target_range, &buf);
        return None;
    }

    return reindent_expr(unsafe_expr, acc, target_range, &buf);
}

pub fn generate_from_utf8(mcall: &CallExpr, expr: &BinExpr) -> Option<String> {

    // Obtain the variable Expr that presents the string
    let receiver = mcall.arg_list()?.args().nth(0)?;

    let pat = expr.lhs()?;

    let mut buf = String::new();

    format_to!(buf, "{} = str::from_utf8({}).unwrap();", pat, receiver);

    buf.push('\n');

    return Some(buf);
}

pub fn generate_let_from_utf8(mcall: &CallExpr, let_expr: &LetStmt) -> Option<String> {

    // Obtain the variable Expr that presents the string
    let receiver = mcall.arg_list()?.args().nth(0)?;

    let pat = let_expr.pat()?;

    let mut buf = String::new();

    format_to!(buf, "let {} = str::from_utf8({}).unwrap();", pat, receiver);

    buf.push('\n');

    return Some(buf);
}

fn convert_to_from_utf8(acc: &mut Assists, target_expr: &SyntaxNode, unsafe_range: TextRange, unsafe_expr: &BlockExpr) -> Option<()> {

    let mcall = target_expr.parent().and_then(ast::CallExpr::cast)?;

    if mcall.syntax().parent()?.kind() == BIN_EXPR {
        let target_expr = mcall.syntax().parent().and_then(ast::BinExpr::cast)?;

        let mut target_range = target_expr.syntax().parent()?.text_range();

        let buf = generate_from_utf8(&mcall, &target_expr)?;
        
        if check_single_bin_expr(&target_expr)? == true {
            target_range = unsafe_range;
            replace_source_code(acc, target_range, &buf);
            return None;
        }
        return reindent_expr(unsafe_expr, acc, target_range, &buf);
    }

    let let_expr = mcall.syntax().parent().and_then(ast::LetStmt::cast)?;

    let buf = generate_let_from_utf8(&mcall, &let_expr)?;

    let mut target_range = let_expr.syntax().text_range();
    if check_single_let_expr(&let_expr) {
        target_range = unsafe_range;
        replace_source_code(acc, target_range, &buf);
        return None;
    }

    return reindent_expr(unsafe_expr, acc, target_range, &buf);
}

pub fn generate_from_transmute(mcall: &CallExpr, let_expr: &LetStmt) -> Option<String> {

    // Obtain the variable Expr that presents the string
    let receiver = mcall.arg_list()?.args().nth(0)?;

    let pat = let_expr.pat()?;

    let mut buf = String::new();

    if let_expr.to_string().contains(&TargetTypes::String.to_string()) {
        format_to!(buf, "let {} = str::from_utf8({}).unwrap();", pat, receiver);
    }

    buf.push('\n');

    return Some(buf);
}

fn transmute_convertion(acc: &mut Assists, target_expr: &SyntaxNode, unsafe_range: TextRange, unsafe_expr: &BlockExpr) -> Option<()> {

    let mcall = target_expr.parent().and_then(ast::CallExpr::cast)?;

    let let_expr = mcall.syntax().parent().and_then(ast::LetStmt::cast)?;

    let buf = generate_from_transmute(&mcall, &let_expr)?;

    let mut target_range = let_expr.syntax().text_range();
    if check_single_let_expr(&let_expr) {
        target_range = unsafe_range;
        replace_source_code(acc, target_range, &buf);
        return None;
    }

    return reindent_expr(unsafe_expr, acc, target_range, &buf);
}

struct CpyNonOverlapInfo {
    src_expr: IndexExpr,
    dst_expr: IndexExpr,
    count: String,
}

fn extract_index_expr(backward_list: impl Iterator<Item = SyntaxNode>, expr_variable: &String) -> Option<IndexExpr>{
    for iter in backward_list {
        if iter.kind() == LET_STMT && iter.to_string().contains(expr_variable) {
            let let_expr = ast::LetStmt::cast(iter)?;
            let expr = ast::IndexExpr::cast(let_expr.syntax().last_child()?.first_child()?)?;
            return Some(expr);
        }
    }
    return None;
}

fn collect_cpy_nonoverlap_info(mcall: &CallExpr, unsafe_expr: &BlockExpr) -> Option<CpyNonOverlapInfo> {

    let src_expr;

    let dst_expr;

    let mut backward_list_src = unsafe_expr.syntax().siblings(Direction::Prev);

    let mut backward_list_dst = unsafe_expr.syntax().siblings(Direction::Prev);

    if unsafe_expr.syntax().parent()?.kind() != STMT_LIST {
        backward_list_src = unsafe_expr.syntax().parent()?.siblings(Direction::Prev);
        backward_list_dst = unsafe_expr.syntax().parent()?.siblings(Direction::Prev);
    }

    if mcall.arg_list()?.args().nth(0)?.syntax().kind() == PATH_EXPR {
        src_expr = extract_index_expr(&mut backward_list_src, &mcall.arg_list()?.args().nth(0)?.to_string())?;
    } else {
        src_expr = ast::IndexExpr::cast(mcall.arg_list()?.args().nth(0)?.syntax().children().nth(0)?)?;
    }

    if mcall.arg_list()?.args().nth(1)?.syntax().kind() == PATH_EXPR {
        dst_expr = extract_index_expr(&mut backward_list_dst, &mcall.arg_list()?.args().nth(1)?.to_string())?;
    } else {
        dst_expr = ast::IndexExpr::cast(mcall.arg_list()?.args().nth(1)?.syntax().children().nth(0)?)?;
    }

    let count = mcall.arg_list()?.args().nth(2)?.to_string();

    return Some(CpyNonOverlapInfo {src_expr, dst_expr, count});
}

fn format_index_expr(expr_index: &IndexExpr, count: &String) -> Option<String> {

    let index = expr_index.index()?.to_string();

    let index_content = index.split("..");

    let vec: Vec<&str> = index_content.collect();

    let lhs = vec[0];
    // let rhs = vec[1];

    let mut buf = String::new();

    if lhs.is_empty() {
        format_to!(buf, "{}[{}..{}]", expr_index.base()?, lhs.to_string(), count);
    } else {
        format_to!(buf, "{}[{}..{} + {}]", expr_index.base()?, lhs.to_string(), lhs.to_string(), count);
    }

    buf.push('\n');

    return Some(buf);

}

pub fn generate_copy_from_slice_string(src_expr: IndexExpr, dst_expr: IndexExpr, count: String) -> Option<String> {

    let mut buf = String::new();

    format_to!(buf, "{}.copy_from_slice(&{});", format_index_expr(&dst_expr, &count)?, format_index_expr(&src_expr, &count)?);

    buf.push('\n');

    return Some(buf);

}

pub fn generate_copy_from_slice_format(mcall: &CallExpr, unsafe_expr: &BlockExpr) -> Option<String> {    

    let CpyNonOverlapInfo { src_expr, dst_expr, count} = collect_cpy_nonoverlap_info(&mcall, &unsafe_expr)?;

    let buf = generate_copy_from_slice_string(src_expr, dst_expr, count)?;

    return Some(buf);
}

fn convert_to_copy_from_slice(acc: &mut Assists, target_expr: &SyntaxNode, unsafe_range: TextRange, unsafe_expr: &BlockExpr) -> Option<()> {

    let mcall = target_expr.parent().and_then(ast::CallExpr::cast)?;

    let target_expr = mcall.syntax().parent().and_then(ast::ExprStmt::cast)?;

    let mut target_range = target_expr.syntax().text_range();

    let buf = generate_copy_from_slice_format(&mcall, &unsafe_expr)?;

    if check_single_expr(&target_expr) {
        target_range = unsafe_range;
        replace_source_code(acc, target_range, &buf);
        return None;
    }

    return reindent_expr(unsafe_expr, acc, target_range, &buf);
}

pub fn generate_cstring_new_string(pat: String, input_argument: String, let_sign: bool) -> String {

    let mut buf = String::new();

    if let_sign {
        format_to!(buf, "let {} = CString::new({}).unwrap();", pat, input_argument);
    } else {
        format_to!(buf, "{} = CString::new({}).unwrap();", pat, input_argument);
    }

    buf.push('\n');

    return buf;

}

pub fn generate_cstring_new_format(pat: String, mcall: &CallExpr, let_sign: bool) -> Option<String> {    

    let input_argument = mcall.arg_list()?.args().nth(0)?.to_string();

    let buf = generate_cstring_new_string(pat, input_argument, let_sign);

    return Some(buf);
}

fn check_single_bin_expr(target_expr: &BinExpr) -> Option<bool> {

    // Check if the unsafe bloack only contains one expr
    if target_expr.syntax().parent()?.prev_sibling().is_none() && target_expr.syntax().parent()?.next_sibling().is_none() {
        return Some(true);
    }
    return Some(false);
}

fn convert_to_cstring_new(acc: &mut Assists, target_expr: &SyntaxNode, unsafe_range: TextRange, unsafe_expr: &BlockExpr) -> Option<()> {

    let mcall = target_expr.parent().and_then(ast::CallExpr::cast)?;

    if mcall.syntax().parent()?.kind() == BIN_EXPR {
        let target_expr = mcall.syntax().parent().and_then(ast::BinExpr::cast)?;

        let mut target_range = target_expr.syntax().parent()?.text_range();

        let buf = generate_cstring_new_format(target_expr.lhs()?.to_string(), &mcall, false)?;
        
        if check_single_bin_expr(&target_expr)? == true {
            target_range = unsafe_range;
            replace_source_code(acc, target_range, &buf);
            return None;
        }
        return reindent_expr(unsafe_expr, acc, target_range, &buf);
    }
        
    let target_expr = mcall.syntax().parent().and_then(ast::LetStmt::cast)?;

    let mut target_range = target_expr.syntax().text_range();

    let buf = generate_cstring_new_format(target_expr.pat()?.to_string(), &mcall, true)?;

    if check_single_let_expr(&target_expr) {
        target_range = unsafe_range;
        replace_source_code(acc, target_range, &buf);
        return None;
    }

    return reindent_expr(unsafe_expr, acc, target_range, &buf);
}

pub fn generate_bytes_len_string(pat: String, input_argument: String, let_sign: bool) -> String {

    let mut buf = String::new();

    if let_sign {
        format_to!(buf, "let {} = {}.as_bytes().len();", pat, input_argument);
    } else {
        format_to!(buf, "{} = {}.as_bytes().len();", pat, input_argument);
    }

    buf.push('\n');

    return buf;

}

pub fn generate_bytes_len_format(pat: String, mcall: &CallExpr, let_sign: bool) -> Option<String> {    

    let input_argument = mcall.arg_list()?.args().nth(0)?.syntax().first_child()?.to_string();

    let buf = generate_bytes_len_string(pat, input_argument, let_sign);

    return Some(buf);
}


fn convert_to_cstring_bytes_len(acc: &mut Assists, target_expr: &SyntaxNode, unsafe_range: TextRange, unsafe_expr: &BlockExpr) -> Option<()> {
    
    let mcall = target_expr.parent().and_then(ast::CallExpr::cast)?;

    if mcall.syntax().parent()?.kind() == BIN_EXPR {
        let target_expr = mcall.syntax().parent().and_then(ast::BinExpr::cast)?;

        let mut target_range = target_expr.syntax().parent()?.text_range();

        let buf = generate_bytes_len_format(target_expr.lhs()?.to_string(), &mcall, false)?;
        
        if check_single_bin_expr(&target_expr)? == true {
            target_range = unsafe_range;
            replace_source_code(acc, target_range, &buf);
            return None;
        }
        return reindent_expr(unsafe_expr, acc, target_range, &buf);
    }
        
    let target_expr = mcall.syntax().parent().and_then(ast::LetStmt::cast)?;

    let mut target_range = target_expr.syntax().text_range();

    let buf = generate_bytes_len_format(target_expr.pat()?.to_string(), &mcall, true)?;

    if check_single_let_expr(&target_expr) {
        target_range = unsafe_range;
        replace_source_code(acc, target_range, &buf);
        return None;
    }

    return reindent_expr(unsafe_expr, acc, target_range, &buf);
}

fn uninitialized_vec_analysis(target_expr: &SyntaxNode, unsafe_expr: &BlockExpr) -> Option<bool> {
    // static analysis on unsafe expr's ancestors() and descendants()
    for backward_slice in unsafe_expr.syntax().parent()?.siblings(Direction::Prev) {
        if backward_slice.to_string().contains(&UnsafePattern::SetVecCapacity.to_string()) ||
            backward_slice.to_string().contains(&UnsafePattern::ReserveVec.to_string()) {

                let mcall = target_expr.parent().and_then(ast::MethodCallExpr::cast)?;

                let receiver = mcall.receiver()?;

                for forward_slice in unsafe_expr.syntax().parent()?.siblings(Direction::Next) {
                    if forward_slice.to_string().contains(&receiver.to_string()) 
                        && forward_slice.to_string().contains(&UnsafePattern::WriteVec.to_string()) {
                        return  Some(false);
                    }

                }
            return Some(true);
        }
    }
    return Some(false);
}

pub fn check_convert_type(target_expr: &SyntaxNode, unsafe_expr: &BlockExpr) -> Option<UnsafePattern> {

    if target_expr.to_string() == UnsafePattern::UnitializedVec.to_string() {
        if uninitialized_vec_analysis(&target_expr, &unsafe_expr)? {
            return Some(UnsafePattern::UnitializedVec);
        }
    }

    if target_expr.to_string() == UnsafePattern::CopyWithin.to_string() {
        return Some(UnsafePattern::CopyWithin);
    }

    if target_expr.to_string() == UnsafePattern::GetUncheck.to_string() {
        return Some(UnsafePattern::GetUncheck);
    }

    if target_expr.to_string() == UnsafePattern::GetUncheckMut.to_string() {
        return Some(UnsafePattern::GetUncheckMut);
    }

    if target_expr.to_string() == UnsafePattern::CopyNonOverlap.to_string() {
        return Some(UnsafePattern::CopyNonOverlap);
    }

    if target_expr.to_string() == UnsafePattern::CStringFromVec.to_string() {
        return Some(UnsafePattern::CStringFromVec);
    }

    if target_expr.to_string() == UnsafePattern::CStringLength.to_string() {
        return Some(UnsafePattern::CStringLength);
    }

    if target_expr.to_string() == UnsafePattern::BytesToUTFString.to_string() {
        return Some(UnsafePattern::BytesToUTFString);
    }

    if target_expr.to_string() == UnsafePattern::TransmuteTo.to_string() {
        return Some(UnsafePattern::TransmuteTo);
    }

    return None;

}

struct UnsafeBlockInfo {
    unsafe_expr: BlockExpr,
    unsafe_range: TextRange,
}

fn collect_unsafe_vec_info(ctx: &AssistContext<'_>) -> Option<UnsafeBlockInfo> {

    // Detect the "unsafe" key word
    let unsafe_kw = ctx.find_token_syntax_at_offset(T![unsafe])?;

    // Collect the expressions within the "unsafe" block
    let unsafe_expr = unsafe_kw.parent().and_then(ast::BlockExpr::cast)?;

    let mut unsafe_range = unsafe_expr.syntax().text_range();

    if unsafe_expr.syntax().parent()?.kind() != STMT_LIST {
        unsafe_range = unsafe_expr.syntax().parent()?.text_range();
    }

    return Some(UnsafeBlockInfo {unsafe_expr, unsafe_range});

}

pub(crate) fn convert_unsafe_to_safe(acc: &mut Assists, ctx: &AssistContext<'_>) -> Option<()> {

    let UnsafeBlockInfo { unsafe_expr, unsafe_range} = collect_unsafe_vec_info(ctx)?;

    // Iteration through the "unsafe" expressions' AST
    for target_expr in unsafe_expr.syntax().descendants() {

        let unsafe_type = check_convert_type(&target_expr, &unsafe_expr);
        
        match unsafe_type {
            Some(UnsafePattern::UnitializedVec) => return convert_to_auto_vec_initialization(acc, &target_expr, unsafe_range, &unsafe_expr),
            Some(UnsafePattern::CopyWithin) => return convert_to_copy_within(acc, &target_expr, unsafe_range, &unsafe_expr),
            Some(UnsafePattern::CopyNonOverlap) => return convert_to_copy_from_slice(acc, &target_expr, unsafe_range, &unsafe_expr),
            Some(UnsafePattern::CStringFromVec) => return convert_to_cstring_new(acc, &target_expr, unsafe_range, &unsafe_expr),
            Some(UnsafePattern::CStringLength) => return convert_to_cstring_bytes_len(acc, &target_expr, unsafe_range, &unsafe_expr),
            Some(UnsafePattern::GetUncheckMut) => return convert_to_get_mut(acc, &target_expr, unsafe_range, &unsafe_expr),
            Some(UnsafePattern::GetUncheck) => return convert_to_get_mut(acc, &target_expr, unsafe_range, &unsafe_expr),
            Some(UnsafePattern::BytesToUTFString) => return convert_to_from_utf8(acc, &target_expr, unsafe_range, &unsafe_expr),
            Some(UnsafePattern::TransmuteTo) => return transmute_convertion(acc, &target_expr, unsafe_range, &unsafe_expr),
            None => continue,
            _ => todo!(),
        };
        
    }

    return None;
    
}

#[cfg(test)]
mod tests {
    use crate::tests::check_assist;

    use super::*;

    #[test]
    fn transmute_byte_to_str_1() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let bytes: &[u8] = &[b'r', b'u', b's', b't'];
    
        unsafe$0 {
            let string: &str = mem::transmute(bytes);
            println!("convert string: {:?}", string);
        }
    }
    "#,
                r#"
    fn main() {

        let bytes: &[u8] = &[b'r', b'u', b's', b't'];
        let string = str::from_utf8(bytes).unwrap();


        unsafe$0 {
            
            println!("convert string: {:?}", string);
        }
    }
    "#,
            );
    }

    #[test]
    fn byte_utf_string_1() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let sparkle_heart : &[u8] = &[240, 159, 146, 150];

        let string;

        unsafe$0 {
            string = str::from_utf8_unchecked(&sparkle_heart)
        }
        println!("sparkle_heart: {:?}", string);
    }
    "#,
                r#"
    fn main() {

        let sparkle_heart : &[u8] = &[240, 159, 146, 150];

        let string;

        string = str::from_utf8(&sparkle_heart).unwrap();
        println!("sparkle_heart: {:?}", string);
    }
    "#,
            );
    }

    #[test]
    fn byte_utf_string_2() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let sparkle_heart : &[u8] = &[240, 159, 146, 150];

        unsafe$0 {
            let string = str::from_utf8_unchecked(&sparkle_heart);
        }
        println!("sparkle_heart: {:?}", string);
    }
    "#,
                r#"
    fn main() {

        let sparkle_heart : &[u8] = &[240, 159, 146, 150];

        let string = str::from_utf8(&sparkle_heart).unwrap();
        println!("sparkle_heart: {:?}", string);
    }
    "#,
            );
    }

    #[test]
    fn from_vec_unchecked_1() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let raw = b"Hello, World!".to_vec();

        unsafe$0 {
            let c_string = CString::from_vec_unchecked(raw);
            println!("The C String: {:?}", c_string);
        }
    }
    "#,
                r#"
    fn main() {

        let raw = b"Hello, World!".to_vec();
        let c_string = CString::new(raw).unwrap();
    
        unsafe$0 {

            println!("The C String: {:?}", c_string);
        }
    }
    "#,
            );
    }

    #[test]
    fn from_vec_unchecked_2() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let raw = b"Hello, World!".to_vec();

        unsafe$0 {
            let c_string = CString::from_vec_unchecked(raw);
        }
    }
    "#,
                r#"
    fn main() {

        let raw = b"Hello, World!".to_vec();

        let c_string = CString::new(raw).unwrap();
    
    }
    "#,
            );
    }

    #[test]
    fn from_vec_unchecked_3() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let raw = b"Hello, World!".to_vec();

        let c_string;

        unsafe$0 {
            c_string = CString::from_vec_unchecked(raw);
            println!("The C String: {:?}", c_string);
        }
    }
    "#,
                r#"
    fn main() {

        let raw = b"Hello, World!".to_vec();

        let c_string;
        c_string = CString::new(raw).unwrap();

        unsafe$0 {

            println!("The C String: {:?}", c_string);
        }
    }
    "#,
            );
    }

    #[test]
    fn cstring_len_1() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let raw = b"Hello, World!".to_vec();

        let c_string = CString::new(raw).unwrap();

        unsafe$0 {
            let length = libc::strlen(c_string.as_ptr());
            println!("The C String: {:?}", length);
        }
    }
    "#,
                r#"
    fn main() {

        let raw = b"Hello, World!".to_vec();

        let c_string = CString::new(raw).unwrap();
        let length = c_string.as_bytes().len();

        unsafe$0 {

            println!("The C String: {:?}", length);
        }
    }
    "#,
            );
    }

    #[test]
    fn cstring_len_2() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let raw = b"Hello, World!".to_vec();

        let c_string = CString::new(raw).unwrap();

        let length;

        unsafe$0 {
            length = libc::strlen(c_string.as_ptr());
            println!("The C String: {:?}", length);
        }
        println!("The C String: {:?}", length);
    }
    "#,
                r#"
    fn main() {

        let raw = b"Hello, World!".to_vec();

        let c_string = CString::new(raw).unwrap();

        let length;
        length = c_string.as_bytes().len();

        unsafe$0 {

            println!("The C String: {:?}", length);
        }
        println!("The C String: {:?}", length);
    }
    "#,
            );
    }
    

    #[test]
    fn copy_nonoverlap_1() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let src = vec![1, 2, 3, 4, 5, 6];
        let mut dst = vec![0; 6];

        let len = 2
    
        unsafe$0 {
            ptr::copy_nonoverlapping(src[1..].as_ptr(), dst[2..].as_mut_ptr(), len);
            println!("copied dst vector: {:?}", dst); 
        }
    }
    "#,
                r#"
    fn main() {

        let src = vec![1, 2, 3, 4, 5, 6];
        let mut dst = vec![0; 6];

        let len = 2
        dst[2..2 + len].copy_from_slice(&src[1..1 + len]);
    
        unsafe$0 {

            println!("copied dst vector: {:?}", dst); 
        }
    }
    "#,
            );
    }

    #[test]
    fn copy_nonoverlap_2() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let src = vec![1, 2, 3, 4, 5, 6];
        let mut dst = vec![0; 6];
    
        unsafe$0 {
            ptr::copy_nonoverlapping(src[2..4].as_ptr(), dst[2..4].as_mut_ptr(), src[2..4].len());
        }
    }
    "#,
                r#"
    fn main() {

        let src = vec![1, 2, 3, 4, 5, 6];
        let mut dst = vec![0; 6];

        dst[2..2 + src[2..4].len()].copy_from_slice(&src[2..2 + src[2..4].len()]);

    }
    "#,
            );
    }

    #[test]
    fn copy_nonoverlap_3() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let src = vec![1, 2, 3, 4, 5, 6];
        let mut dst = vec![0; 6];
        let len = 3
        let source = src[1..3].as_ptr();
        let dest = dst[2..3].as_mut_ptr();
    
        unsafe$0 {
            ptr::copy_nonoverlapping(source, dest, len);
        }
    }
    "#,
                r#"
    fn main() {

        let src = vec![1, 2, 3, 4, 5, 6];
        let mut dst = vec![0; 6];
        let len = 3
        let source = src[1..3].as_ptr();
        let dest = dst[2..3].as_mut_ptr();

        dst[2..2 + len].copy_from_slice(&src[1..1 + len]);
    }
    "#,
            );
    }

    #[test]
    fn copy_nonoverlap_4() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let src = vec![1, 2, 3, 4, 5, 6];
        let mut dst = vec![0; 6];
        let len = 3
        let source = src[1..3].as_ptr();
        let dest = dst[2..3].as_mut_ptr();
    
        unsafe$0 {
            ptr::copy_nonoverlapping(source, dest, len);
            println!("copied dst vector: {:?}", dst);
        }
        println!("copied dst vector: {:?}", dst);
    }
    "#,
                r#"
    fn main() {

        let src = vec![1, 2, 3, 4, 5, 6];
        let mut dst = vec![0; 6];
        let len = 3
        let source = src[1..3].as_ptr();
        let dest = dst[2..3].as_mut_ptr();

        dst[2..2 + len].copy_from_slice(&src[1..1 + len]);

        unsafe$0 {

            println!("copied dst vector: {:?}", dst);
        }
        println!("copied dst vector: {:?}", dst);
    }
    "#,
            );
    }

    #[test]
    fn get_uncheckd_1() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let vec = vec![1,2,3,4,5,6];
    
        unsafe$0 {
            let index = vec.get_unchecked(5);    
            print!("Index: {:?} \n", index);
        }
    }
    "#,
                r#"
    fn main() {

        let vec = vec![1,2,3,4,5,6];
        let index = vec.get(5).unwrap();
        unsafe$0 {

            print!("Index: {:?} \n", index);
        }
    }
    "#,
            );
    }

    #[test]
    fn get_uncheckd_2() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let vec = vec![1,2,3,4,5,6];
    
        unsafe$0 {
            let index = vec.get_unchecked(5);    
        }
    }
    "#,
                r#"
    fn main() {

        let vec = vec![1,2,3,4,5,6];
    
        let index = vec.get(5).unwrap();
    }
    "#,
            );
    }

    #[test]
    fn get_uncheckd_mut_1() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let mut vec = vec![1,2,3,4,5,6];
    
        unsafe$0 {
            let index = vec.get_unchecked_mut(5);    
            print!("Index: {:?} \n", index);
        }
    }
    "#,
                r#"
    fn main() {

        let mut vec = vec![1,2,3,4,5,6];
        let index = vec.get_mut(5).unwrap();
        unsafe$0 {

            print!("Index: {:?} \n", index);
        }
    }
    "#,
            );
    }

    #[test]
    fn get_uncheckd_mut_2() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let mut vec = vec![1,2,3,4,5,6];

        let index;
    
        unsafe$0 {
            index = vec.get_unchecked_mut(5);    
        }
        print!("Index: {:?} \n", index);
    }
    "#,
                r#"
    fn main() {

        let mut vec = vec![1,2,3,4,5,6];

        let index;

        index = vec.get_mut(5).unwrap();
        print!("Index: {:?} \n", index);
    }
    "#,
            );
    }

    #[test]
    fn get_uncheckd_mut_3() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let mut vec = vec![1,2,3,4,5,6];
    
        unsafe$0 {
            let index = vec.get_unchecked_mut(5);    
        }
        print!("Index: {:?} \n", index);
    }
    "#,
                r#"
    fn main() {

        let mut vec = vec![1,2,3,4,5,6];

        let index = vec.get_mut(5).unwrap();
        print!("Index: {:?} \n", index);
    }
    "#,
            );
    }

    #[test]
    fn convert_ptr_copy_1() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let mut vec = vec![1,2,3,4,5,6];

        let src = vec[0..].as_mut_ptr();

        let dst = vec[2..].as_mut_ptr();
    
        unsafe$0 {
            ptr::copy(src, dst, 4);
            println!("Hello World!");
        }
        let mut n = 1;
    }
    "#,
                r#"
    fn main() {

        let mut vec = vec![1,2,3,4,5,6];

        let src = vec[0..].as_mut_ptr();

        let dst = vec[2..].as_mut_ptr();

        vec.copy_within(0..4, 2);

        unsafe$0 {
            
            println!("Hello World!");
        }

        let mut n = 1;
    }
    "#,
            );
    }

    #[test]
    fn convert_ptr_copy_2() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let mut vec = vec![1,2,3,4,5,6];
    
        unsafe$0 {
            ptr::copy(&vec[0] as *const i32, &mut vec[2] as *mut i32, 4);
            println!("Hello World!");
        }

        let mut n = 1;
    }
    "#,
                r#"
    fn main() {

        let mut vec = vec![1,2,3,4,5,6];
        vec.copy_within(0..4, 2);

        unsafe$0 {
            
            println!("Hello World!");
        }
    
        let mut n = 1;
    }
    "#,
            );
    }

    #[test]
    fn convert_ptr_copy_3() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let mut vec = vec![1,2,3,4,5,6];

        let dst = vec.len() + heap.size();
    
        unsafe$0 {
            ptr::copy(vec[0..].as_mut_ptr(), vec[3..].as_mut_ptr(), dst);
        }
    }
    "#,
                r#"
    fn main() {

        let mut vec = vec![1,2,3,4,5,6];

        let dst = vec.len() + heap.size();

        vec.copy_within(0..dst, 3);

    }
    "#,
            );
    }
    
    #[test]
    fn convert_vec_1() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let cap = 100;

        let mut buffer = Vec::with_capacity(cap);

        unsafe$0 {
            buffer.set_len(cap); 
            println!("Hello World!");
        }
        println!("Hello World Again!");
    }
    "#,
                r#"
    fn main() {

        let cap = 100;

        let mut buffer = vec![0; cap];

        unsafe$0 {
            
            println!("Hello World!");
        }
        println!("Hello World Again!");
    }
    "#,
            );
    }

    #[test]
    fn convert_vec_2() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let cap = 100;

        let mut buffer = Vec::with_capacity(cap);

        unsafe$0 {
            buffer.set_len(cap); 
        }
        input.read_into(&mut buffer);
        println!("Hello World Again!");
    }
    "#,
                r#"
    fn main() {

        let cap = 100;

        let mut buffer = vec![0; cap];
        
        input.read_into(&mut buffer);
        println!("Hello World Again!");
    }
    "#,
            );
    }

    #[test]
    fn convert_vec_3() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let len = 100;

        let mut buf = Vec::<u32>::with_capacity(len as usize); 
        unsafe$0 { buf.set_len(len as usize) };
    
    }
    "#,
                r#"
    fn main() {

        let len = 100;

        let mut buf = vec![0; len as usize];
    }
    "#,
            );
    }

    #[test]
    fn convert_vec_4() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let len = 100;

        let mut buf = vec![0; 10];
        
        buf.reserve(len); 

        unsafe$0 { 
            buf.set_len(len); 
        } 
    }
    "#,
                r#"
    fn main() {

        let len = 100;

        let mut buf = vec![0; 10];

        buf.resize(len, 0);
        
    }
    "#,
            );
    }
}
