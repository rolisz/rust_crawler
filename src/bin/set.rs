use std::collections::HashSet;

fn main() {
    let mut books : HashSet<String> = HashSet::new();
    let v = vec!("Bible".to_string(), "8 biographis".into(), "NCL".into(), "Bible".into());
    println!("{:#?}", v);
//    for b in v {
//        books.insert(b);
//    }
    println!("{:#?}", books);

    v.iter().map(|x| {
        println!("{}", x);
        books.insert(x.to_string())
    }).collect::<Vec<bool>>();
        println!("{:#?}", books);

}