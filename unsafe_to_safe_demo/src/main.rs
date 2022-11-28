// fn main() {

//     let len = 100;

//     let mut buffer : Vec<i32> = Vec::with_capacity(len);
        
//     unsafe { 
//         buffer.set_len(len);
//     }
//     println!("buffer length: {:?}", buffer.len());
// }

// fn main() {

//     let len = 100;

//     let mut buf = vec![0; 10];
    
//     buf.reserve(len); 

//     unsafe { 
//         buf.set_len(len); 
//     }
//     println!("buffer length: {:?}", buf.len()); 
// }

// use std::ptr;
// fn main() {

//     let mut vec = vec![1,2,3,4,5,6];

//     println!("original vector: {:?}", vec);

//     let cnt = 10;

//     let src = &vec[0] as *const i32;

//     let mut dst = &mut vec[2] as *mut i32;

//     unsafe {
//         ptr::copy(src, dst, cnt);
//     }
//     println!("copied vector: {:?}", vec);
// }

// use std::ptr;
// fn main() {

//     let src = vec![1, 2, 3, 4, 5, 6];

//     let mut dst = vec![0; 6];

//     println!("original dst vector: {:?}", dst); 

//     let cnt = 10;

//     let source = src[2..].as_ptr();
    
//     let dest = dst[2..].as_mut_ptr();

//     unsafe {
//         ptr::copy_nonoverlapping(source, dest, cnt);
//     }
//     println!("copied dst vector: {:?}", dst);
// }

// use std::ffi::CString;
// fn main() {

//     let raw = b"Hello, World!".to_vec();

//     let c_string;

//     unsafe {
//         c_string = CString::from_vec_unchecked(raw);
//     }
//     println!("The C String: {:?}", c_string);
// }

// use std::ffi::CString;
// fn main() {

//     let raw = b"Hello, World!".to_vec();

//     let c_string = CString::new(raw).unwrap();

//     let length;

//     unsafe {
//         length = libc::strlen(c_string.as_ptr());
//         println!("The C String: {:?}", length);
//     }
//     println!("The C String: {:?}", length);
// }

// fn main() {

//     let mut vec = vec![1,2,3,4,5,6];

//     let index;

//     unsafe {
//         index = vec.get_unchecked_mut(5);    
//         print!("Index: {:?} \n", index);
//     }

//     print!("Index: {:?} \n", index);
// }

// fn main() {

//     let vec = vec![1,2,3,4,5,6];

//     let index;

//     unsafe {
//         index = vec.get_unchecked(5);    
//         print!("Index: {:?} \n", index);
//     }

//     print!("Index: {:?} \n", index);
// }

// use std::str;
// fn main() {

//     // some bytes, in a vector
//     let sparkle_heart : &[u8] = &[240, 159, 146, 150];

//     // let invalid : &[u8] = &[159, 159, 146, 146];

//     let string;

//     unsafe {
//         string = str::from_utf8_unchecked(sparkle_heart)
//     }

//     println!("sparkle_heart: {:?}", string);
    
// }

// use std::str;
// use std::mem;

// fn main() {

//     let bytes: &[u8] = &[b'r', b'u', b's', b't'];
//     unsafe {
//         let string: &str = mem::transmute(bytes);
//         println!("convert string: {:?}", string);
//     }
//     println!("original bytes: {:?}", bytes);
// }

// use std::mem;

// fn main() {

//     let string = "rust";
//     unsafe {
//         let bytes: &[u8] = mem::transmute(string);
//         println!("convert bytes: {:?}", bytes);
//     }
//     // let bytes = string.as_bytes();
//     println!("original string: {:?}", string);
// }

// use std::mem;
// fn main() {

//     let float: f32 = 1.23;
//     unsafe {
//         let int: u32 = mem::transmute(float);
//         println!("convert int: {:?}", int);
//     }  
//     println!("original float: {:?}", float);

//     let float: f64 = 1.23;
//     unsafe {
//         let int: u64 = mem::transmute(float);
//         println!("convert int: {:?}", int);
//     }  
//     println!("original float: {:?}", float);
// }

// use std::mem;
// fn main() {

//     let int: u64 = 666;
//     unsafe {
//         let float: f64 = mem::transmute(int);
//         println!("convert float: {:?}", float);
//     }  
//     let float = f64::from_bits(int);
//     println!("convert float: {:?}", float);

//     let int_32: u32 = 666;
//     unsafe {
//         let float_32: f32 = mem::transmute(int_32);
//         println!("convert float: {:?}", float_32);
//     }  
//     let float_32 = f32::from_bits(int_32);
//     println!("convert float: {:?}", float_32);

// }

// use std::mem;
// fn main() {

//     let int: u32 = 65;
//     unsafe {
//         let letter: char = mem::transmute(int);
//         println!("convert letter: {:?}", letter);
//     }  

//     let letter: char = char::from_u32(int).unwrap();
//     println!("convert letter: {:?}", letter);

//     let float: f32 = 1.23;
//     let letter: char = 'A';
//     unsafe {
//         let letter_int: u32 = mem::transmute(letter);
//         let int: u32 = mem::transmute(float);
//         println!("convert float to int: {:?}", int);
//         println!("convert letter to int: {:?}", letter_int);
//     }  

//     let int: u32 = letter as u32;
//     println!("convert int: {:?}", int);

// }

use std::ptr;

fn main() {

    let bytes: &[u8] = &[6, 7, 8, 4, 5, 6];

    let ptr = bytes.as_ptr();
    unsafe { 
        let int = ptr::read_unaligned(ptr as *const u8);
        println!("The convert int: {:?}", int);
    }

    let int = u8::from_ne_bytes(bytes[..1].try_into().unwrap());
    println!("The convert int: {:?}", int);

    let bytes: &[u8] = &[6, 7, 8, 4, 5, 6];
    unsafe { 
        let int = ptr::read_unaligned(bytes.as_ptr() as *const u16);
        println!("The convert int: {:?}", int);
    }

    let int = u16::from_ne_bytes(bytes[..2].try_into().unwrap());
    println!("The convert int: {:?}", int);

    let bytes: &[u8] = &[1, 2, 3, 4, 5, 6];
    let int;
    unsafe { 
        int = ptr::read_unaligned(bytes.as_ptr() as *const u32);
        println!("The convert int: {:?}", int);
    }

    let int = u32::from_ne_bytes(bytes[..4].try_into().unwrap());
    println!("The convert int: {:?}", int);

    let bytes: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8];
    let int;
    unsafe { 
        int = ptr::read_unaligned(bytes.as_ptr() as *const u64);
        println!("The convert int: {:?}", int);
    }

    let int = u64::from_ne_bytes(bytes[..8].try_into().unwrap());
    println!("The convert int: {:?}", int);
    
}
