use crate::{
    assist_context::{AssistContext, Assists},
    AssistId, AssistKind,
};

use syntax::{SyntaxKind::INDEX_EXPR, ast::{IndexExpr, BlockExpr, MethodCallExpr, ExprStmt}};
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

// Assist: convert_while_to_loop
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
    UnitializedVec,
}

impl std::fmt::Display for UnsafePattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnsafePattern::UnitializedVec => write!(f, "set_len"),
        }
    }
}

pub fn generate_safevec_format(mcall: &MethodCallExpr) -> Option<String> {

    // Obtain the variable Expr that presents the buffer/vector
    let receiver = mcall.receiver()?;

    let closure_body = mcall.arg_list()?.args().exactly_one().ok()?;

    let mut buf = String::new();

    format_to!(buf, "let mut {} = vec![0; {}];", receiver, closure_body);

    return Some(buf);

}

fn check_single_expr(target_expr: &ExprStmt) -> bool {

    // Check if the unsafe bloack only contains one expr
    if target_expr.syntax().prev_sibling().is_none() && target_expr.syntax().next_sibling().is_none() {
        return true;
    }
    return false;
}

fn modify_source_code(acc: &mut Assists, target_expr: &SyntaxNode, target_range: TextRange, buf: &String) -> Option<bool> {

    if target_expr.to_string().contains("Vec::with_capacity") {
            
        for iter in target_expr.descendants() {
            if iter.to_string() == "Vec::with_capacity" {
                let prev_mcall = iter.parent().and_then(ast::Expr::cast)?;

                let let_expr = prev_mcall.syntax().parent().and_then(ast::LetStmt::cast)?;
                
                let let_target = let_expr.syntax().text_range();
                // Delete the "set_len" expression in unsafe code block and insert the auto initialized vec/buf
                acc.add(
                    AssistId("convert_unsafe_to_safe", AssistKind::RefactorRewrite),
                    "Convert Unsafe to Safe",
                    target_range,
                    |edit| {
                        edit.delete(target_range);
                        edit.replace(let_target, buf)
                    },
                );
                return Some(true);
            }
        }
    }

    return Some(false);

}

fn convert_to_auto_vec_initialization(acc: &mut Assists, target_expr: &SyntaxNode, unsafe_range: TextRange) -> Option<()> {

    let mcall = target_expr.parent().and_then(ast::MethodCallExpr::cast)?;

    let buf = if let Some(buffer) = generate_safevec_format(&mcall) {buffer} else { return None; };

    // Declare the target text range for modification.
    let target_expr = mcall.syntax().parent().and_then(ast::ExprStmt::cast)?;

    let mut target_range = target_expr.syntax().text_range();
    if check_single_expr(&target_expr) {
        target_range = unsafe_range;
    }

    for iter in mcall.syntax().ancestors() {

        if modify_source_code(acc, &iter, target_range, &buf)? == true {
            break;
        }
        continue;
    }
    return None;
}

fn convert_to_copy_within(acc: &mut Assists, target_expr: &SyntaxNode, unsafe_range: TextRange) -> Option<()> {

    let mcall = target_expr.parent().and_then(ast::CallExpr::cast)?;

    // let base;

    // let start_pos;

    // let end_pos;

    // let offset;

    for i in mcall.arg_list()?.args() {
        println!("function expr: {:?}", i.syntax().to_string());
        for j in i.syntax().children() {
            println!("child expr: {:?}", j.to_string());
            for x in j.children() {
                if x.kind() == INDEX_EXPR {
                    let y = ast::IndexExpr::cast(x)?;

                    for a in y.to_string().split('[').nth(0) {
             
                        println!("child child expr: {:?}", a);

                    }
                }
                
                // dbg!(x);
            }
        
        }
    }

    // let args = mcall.arg_list()?.args().next()?;

    // println!("function expr: {:?}", args.syntax().to_string());
    

    // dbg!(target_expr.parent());
    return None;

}

struct UnsafeBlockInfo {
    unsafe_expr: BlockExpr,
    unsafe_range: TextRange,
}

fn collect_unsafe_info(ctx: &AssistContext<'_>) -> Option<UnsafeBlockInfo> {

    // Detect the "unsafe" key word
    let unsafe_kw = ctx.find_token_syntax_at_offset(T![unsafe])?;

    // Collect the expressions within the "unsafe" block
    let unsafe_expr = unsafe_kw.parent().and_then(ast::BlockExpr::cast)?;

    let unsafe_range = unsafe_expr.syntax().text_range();

    return Some(UnsafeBlockInfo {unsafe_expr, unsafe_range});

}

pub(crate) fn convert_unsafe_to_safe(acc: &mut Assists, ctx: &AssistContext<'_>) -> Option<()> {

    let (unsafe_expr, unsafe_range) = 
        if let Some(UnsafeBlockInfo {unsafe_expr, unsafe_range}) = collect_unsafe_info(ctx) { 
            (unsafe_expr, unsafe_range) 
        } else { return None; };

    // Iteration through the "unsafe" expressions' AST
    for target_expr in unsafe_expr.syntax().descendants() {

        // Detect the first pattern "vec/buf declared, but without initialization" in unsafe code block
        if target_expr.to_string() == UnsafePattern::UnitializedVec.to_string() {
            // Convert first pattern to safe code by calling auto initialization function
            convert_to_auto_vec_initialization(acc, &target_expr, unsafe_range);
        }

        // Detect the second pattern "ptr::copy" in unsafe code block
        // if target_expr.to_string() == "ptr::copy" {
        //     // Convert second pattern to safe code by calling "copy_within"
        //     convert_to_copy_within(acc, &target_expr, unsafe_range);
        // }
        
    }

    return None;
    
}

#[cfg(test)]
mod tests {
    use crate::tests::check_assist;

    use super::*;

    #[test]
    fn convert_ptr_copy_1() {
        check_assist(
            convert_unsafe_to_safe,
            r#"
    fn main() {

        let mut vec = vec![1,2,3,4,5,6];
    
        unsafe$0 {
            ptr::copy(&vec[0] as *const i32, &mut vec[3] as *mut i32, 3);
        }
    }
    "#,
                r#"
    fn main() {

        let mut vec = vec![1,2,3,4,5,6];

        vec.copy_within(0..3, 3);
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
    }
    "#,
                r#"
    fn main() {

        let cap = 100;

        let mut buffer = vec![0; cap];

        unsafe$0 {
            
            println!("Hello World!");
        }
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
    }
    "#,
                r#"
    fn main() {

        let cap = 100;

        let mut buffer = vec![0; cap];

    }
    "#,
            );
    }
}