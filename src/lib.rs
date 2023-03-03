// use proc_macro:;
use proc_macro2::*;
use quote::quote;

#[cfg(not(feature = "auto-sys"))]
// #[proc_macro_attribute]
fn auto_system(){
    // return input;
}
#[cfg(feature = "auto-sys")]
#[proc_macro_attribute]
pub fn auto_system(_at:proc_macro::TokenStream,input:proc_macro::TokenStream)->proc_macro::TokenStream{
    //scan the token tree for:
    //spawn!()
    
    let input_r:TokenStream = input.into();
    let mut iter = input_r.into_iter().peekable();
    while let Some(e) = iter.peek() {
        if !part_of_function_identity(&e) {
            break;
        }
        let _ = iter.next();
    } 
    let name = iter.next().expect("name");
    let params = iter.next().expect("params");

    // dbg!(&name);
    // dbg!(&params);
    let body = iter.next().expect("body");

    let (flags,transformed_body) = token_return_flags(&body);
    let transformed_body = TokenStream::from_iter(transformed_body);
    let param = TokenStream::from_iter(flags.build());
    return quote!{
        pub fn #name (#param) {#transformed_body}
    }.into();
}

#[derive(Default)]
struct Flags {
    mut_commands:bool,
    asset_server:bool,
    time:bool,
    windows:bool,
    queries:Vec<(String,ForloopContext)>,
    custom_resources:Vec<String>,
    custom_resources_mut:Vec<String>,
}

struct SystemQuery{
    pub params:Vec<String>,
    pub with:Vec<String>,
}
impl SystemQuery {
    pub fn new(from:&String)->Self{
        let ident = query_to_ident(from);
        let mut with:Vec<String> = vec![];
        let mut params:Vec<String> = vec![];
        let mut words = from.split(" ").peekable();
        let mut context = Context::ParamContext;
        while let Some(word) = words.next() {
            match word {
                "with"=>{
                    context = Context::WithContext;
                    let t = words.next().expect("query after WITH keyword");
                    with.push(t.to_string());
                },
                "and"|","=>{
                    let t = words.next().expect("query after WITH keyword");
                    match context {
                        Context::WithContext =>  with.push(t.to_string()),
                        Context::ParamContext => params.push(t.to_string()),
                    }
                }
                _=> {
                    params.push(word.to_string());
                }
            }
        }
        return Self {
            params,
            with,
        }
    }
}

enum Context {
    WithContext,
    ParamContext
}
impl Flags {
    pub fn join(&mut self,with:Flags){
        self.mut_commands = self.mut_commands | with.mut_commands;
        self.asset_server = self.asset_server | with.asset_server;
        self.time = self.time | with.time;
        self.windows = self.windows | with.windows;
        self.queries.extend(with.queries);
        for rs in with.custom_resources {
            self.add_res(rs);
        }
        for rs in with.custom_resources_mut {
            self.add_res_mut(rs);
        }
    }
    pub fn add_res(&mut self,rs:String){
        if !self.custom_resources.contains(&rs) {
            self.custom_resources.push(rs);
        }
    }
    pub fn add_res_mut(&mut self,rs:String){
        if !self.custom_resources_mut.contains(&rs) {
            self.custom_resources_mut.push(rs);
        }
    }
    pub fn build(&self)->Vec<TokenTree>{
        let mut stream_list:Vec<TokenStream> = vec![];
        let mut tokens:Vec<TokenTree> = vec![];
        if self.mut_commands {
            stream_list.push(quote!(mut commands:Commands));
        }
        if self.asset_server {
            stream_list.push(quote!(assets:Res<AssetServer>));
        }
        if self.time {
            stream_list.push(quote!(time:Res<Time>));
        }
        if self.windows {
            stream_list.push(quote!(windows:Res<Windows>));
        }
        if self.custom_resources.len() > 0 {
            let res_it = self.custom_resources.iter().map(|e|{
                let id = to_ident(e);
                let nid = to_ident(&format!("resource_{}",e.to_lowercase()));
                quote!(#nid:Res<#id>)
            });
            stream_list.push(quote!(#(#res_it),*));
        }
        if self.custom_resources_mut.len() > 0 {
            let res_it = self.custom_resources_mut.iter().map(|e|{
                let id = to_ident(e);
                let nid = to_ident(&format!("resource_mut_{}",e.to_lowercase()));
                quote!(mut #nid:ResMut<#id>)
            });
            stream_list.push(quote!(#(#res_it),*));
        }
        for q in self.queries.iter() {
            let ident = query_to_ident(&q.0);
            let mut with:Vec<String> = vec![];
            let mut params:Vec<String> = vec![];
            let mut words = q.0.split(" ").peekable();
            let mut context = Context::ParamContext;
            while let Some(word) = words.next() {
                match word {
                    "with"=>{
                        context = Context::WithContext;
                        let t = words.next().expect("query after WITH keyword");
                        with.push(t.to_string());
                    },
                    _=> {
                        params.push(word.to_string());
                    }
                }
            }
            let additional = match with.len() {
                0 => TokenStream::new(),
                1 => {
                    let i = to_ident(with.first().unwrap());
                    quote!(,With<#i>)
                },
                _ => {
                    let i_iter = with.iter().map(to_ident);
                    quote!(,With<(#(#i_iter),*)>)
                }
            };
            let param_iter = params.iter().map(to_ident).enumerate().map(|(index,e)|{
                match &q.1 {
                    ForloopContext::None => quote!(&#e),
                    ForloopContext::STD => quote!(&#e),
                    ForloopContext::Mutable(is_mut) => if is_mut[index] {quote!(&mut #e) }else{quote!(&#e)},
                }
            });
            if params.len() > 1 {
                match q.1  {
                    ForloopContext::Mutable(_) => stream_list.push(quote!(mut #ident:Query<(#(#param_iter),*)#additional>)),
                    _=>stream_list.push(quote!(#ident:Query<(#(#param_iter),*)#additional>))
                };
            }else{
                match q.1  {
                    ForloopContext::Mutable(_) =>  stream_list.push(quote!(mut #ident:Query<#(#param_iter),*#additional>)),
                    _=> stream_list.push(quote!(#ident:Query<#(#param_iter),*#additional>))
                };
            }
        }

        if stream_list.len() == 0{
            return tokens;
        }
        let last = stream_list.len()-1;
        for (index,stream) in stream_list.iter().cloned().enumerate() {
            tokens.extend(stream.into_iter());
            if index != last {
                tokens.push(TokenTree::Punct(Punct::new(',', Spacing::Joint)));
            }
        }
        return tokens;
    }
}

fn to_ident(string:&String)->TokenTree{
    TokenTree::Ident(Ident::new(string.as_str(), Span::call_site()))
}
fn to_ident_str(string:&str)->TokenTree{
    TokenTree::Ident(Ident::new(string, Span::call_site()))
}
fn is_mut(token:&TokenTree)->bool{
    match token {
        TokenTree::Ident(e) => e.to_string()=="mut",
        _ => false,
    }
}
fn is_comma(token:&TokenTree)->bool{
    match token {
        TokenTree::Punct(e) => e.as_char()==',',
        _ => false,
    }
}
#[derive(Clone)]
enum ForloopContext{
    None,
    STD,
    Mutable(Vec<bool>)
}

fn token_return_flags(token:&TokenTree)->(Flags,Vec<TokenTree>){
    let mut flags = Flags::default();
    // dbg!(&token);
    let mut tokens:Vec<TokenTree> = vec![];
    let mut context = ForloopContext::None;
    match token {
        TokenTree::Group(g) => {
            let mut iter = g.stream().into_iter().peekable();
            while let Some(t) = iter.next() {
                // dbg!(&t);
                match t {
                    TokenTree::Group(f) => {
                        let (o_flags,o_tok) = token_return_flags(&TokenTree::Group(Group::new(Delimiter::Bracket, f.stream())));
                        flags.join(o_flags);
                        tokens.push(TokenTree::Group(Group::new(f.delimiter(), TokenStream::from_iter(o_tok))));
                        continue;
                    },
                    TokenTree::Ident(e) => {
                        let Some(n) = iter.peek() else {tokens.push(TokenTree::Ident(e));continue;};
                        if !is_exclamation_mark(&n){
                            // dbg!(&e);
                            match e.to_string().as_str() {
                                "for"=>{
                                    let ctx = match iter.peek().unwrap() {
                                        TokenTree::Group(col) => {
                                            let mut mut_list = vec![];
                                            let mut pushed_bool=false;
                                            for e in col.stream().into_iter() {
                                                if is_mut(&e) {
                                                    pushed_bool = true;
                                                    mut_list.push(true);
                                                    pushed_bool = false;
                                                }else if(is_comma(&e)){
                                                    if !pushed_bool {
                                                        mut_list.push(false);
                                                        pushed_bool = false;
                                                    }
                                                }
                                            }
                                            if mut_list.contains(&true) {
                                                ForloopContext::Mutable(mut_list)
                                            }else{
                                                ForloopContext::STD
                                            }
                                        },
                                        TokenTree::Ident(e) => if e.to_string()=="mut" {ForloopContext::Mutable(vec![true])}else{ForloopContext::STD},
                                        _=>ForloopContext::STD,
                                    };
                                    context = ctx;
                                    tokens.push(TokenTree::Ident(e));
                                },
                                _=>{
                                    tokens.push(TokenTree::Ident(e));
                                }
                            }
                           
                            continue;
                        }
                        match e.to_string().as_str() {
                            "spawn"=>{
                                flags.mut_commands=true;
                                tokens.push(to_ident_str("internal_spawn"));
                            },
                            "load"=>{
                                flags.asset_server=true;
                                tokens.push(to_ident_str("internal_load"));
                            },
                            "time"=>{
                                flags.time=true;
                                tokens.push(to_ident_str("internal_time"));
                            },
                            "delta_seconds"=>{
                                flags.time=true;
                                tokens.push(to_ident_str("internal_delta_seconds"));
                            },
                            "delta_time"=>{
                                flags.time=true;
                                tokens.push(to_ident_str("internal_delta_seconds"));
                            },
                            "windows"=>{
                                flags.windows=true;
                                tokens.push(to_ident_str("internal_windows"));
                            },
                            "res"|"resource"=>{
                                let _ = iter.next();
                                let g = iter.peek().unwrap();
                                match g {
                                    TokenTree::Group(a)=>{
                                        let mut c:Vec<TokenTree> = a.stream().into_iter().collect();
                                        if is_mut(c.first().unwrap()) {
                                            flags.add_res_mut(TokenStream::from_iter((&c[1..]).iter().cloned()).to_string());
                                        }else{
                                            flags.add_res(a.stream().to_string());
                                        }
                                    },
                                    _=>{}
                                };
                                tokens.push(to_ident_str("internal_resource"));
                                tokens.push(TokenTree::Punct(Punct::new('!', Spacing::Alone)));
                            }
                            "query"=>{
                                let _ = iter.next();
                                let body = iter.next().expect("query body");
                                match body {
                                    TokenTree::Group(h)=>{
                                        flags.queries.push((h.stream().to_string(),context.clone()));
                                        match &context {
                                            ForloopContext::None => tokens.push(to_ident_str("internal_untyped_query")),
                                            ForloopContext::STD => tokens.push(to_ident_str("internal_query")),
                                            ForloopContext::Mutable(_) => tokens.push(to_ident_str("internal_mut_query")),
                                        }
                                        context = ForloopContext::None;
                                        tokens.push(TokenTree::Punct(Punct::new('!', Spacing::Alone)));
                                        tokens.push(TokenTree::Group(h));
                                    },
                                    _=>{}
                                }
                                
                            }
                            _=>{
                                // dbg!(&e);
                                tokens.push(TokenTree::Ident(e))
                            }
                        }
                        continue;
                    },
                    _=>{
                        tokens.push(t)
                    }
                }
                
            }
        },
        _=>{

        }
        
    }
    // dbg!(&tokens);
    println!("ts: {}",TokenStream::from_iter(tokens.clone()).to_string());
    return (flags,tokens)
}

// fn string_value(t:&TokenTree)->String{
//     match t {
//         TokenTree::Ident(e)=>e.to_string(),
//         _=>"".to_string()
//     }
// }

fn is_exclamation_mark(t:&TokenTree)->bool{
    match t{
        TokenTree::Punct(e)=>e.as_char()=='!',
        _=>false
    }
}

fn part_of_function_identity(t:&TokenTree)->bool{
    match t{
        TokenTree::Ident(e) => ["pub","fn"].contains(&e.to_string().as_str()),
        _=>false
    }
}

fn query_to_ident(q:&String)->TokenTree{
    let out = format!("query_{}",q.split(" ").map(|e|e.to_ascii_lowercase()).collect::<Vec<String>>().join("_"));
    let ident = TokenTree::Ident(Ident::new(out.as_str(), Span::call_site()));
    return ident
}

// fn to_ident(f:&str)->TokenTree{
//     let ident = TokenTree::Ident(Ident::new(f, Span::call_site()));
//     return ident
// }
#[proc_macro]
pub fn query(input:proc_macro::TokenStream)->proc_macro::TokenStream{
    let q = input.to_string();
    // let ident = query_to_ident(&q);
    // return quote!(#ident.iter()).into();
    //make the compiler happy
    let s = SystemQuery::new(&q);
    let idents = s.params.iter().map(to_ident);
    if s.params.len() > 1 {
        quote!(&mut [] as &mut [(#(#idents),*)]).into()
    }else{
        quote!(&mut [] as &mut [#(#idents),*]).into()
    }
}

#[proc_macro]
pub fn internal_untyped_query(input:proc_macro::TokenStream)->proc_macro::TokenStream{
    let q = input.to_string();
    let ident = query_to_ident(&q);
    return quote!(#ident).into();
}
#[proc_macro]
pub fn internal_query(input:proc_macro::TokenStream)->proc_macro::TokenStream{
    let q = input.to_string();
    let ident = query_to_ident(&q);
    return quote!(#ident.iter()).into();
}
#[proc_macro]
pub fn internal_mut_query(input:proc_macro::TokenStream)->proc_macro::TokenStream{
    let q = input.to_string();
    let ident = query_to_ident(&q);
    return quote!(#ident.iter_mut()).into();
}

#[proc_macro]
pub fn internal_spawn(input:proc_macro::TokenStream)->proc_macro::TokenStream{
    let i:TokenStream = input.into();
    return quote!(commands.spawn(#i)).into();
}
#[proc_macro]
pub fn internal_load(input:proc_macro::TokenStream)->proc_macro::TokenStream{
    let i:TokenStream = input.into();
    return quote!(assets.load(#i)).into();
}
#[proc_macro]
pub fn internal_time(input:proc_macro::TokenStream)->proc_macro::TokenStream{
    let i:TokenStream = input.into();
    return quote!(time).into();
}
#[proc_macro]
pub fn internal_delta_seconds(input:proc_macro::TokenStream)->proc_macro::TokenStream{
    let i:TokenStream = input.into();
    return quote!((time.delta_seconds())).into();
}
#[proc_macro]
pub fn spawn(input:proc_macro::TokenStream)->proc_macro::TokenStream{
    // let i:TokenStream = input.into();
    return quote!(
        bevy::ecs::system::EntityCommands{
            entity: todo!(),
            commands: todo!(),
        }
    ).into();
}
#[proc_macro]
pub fn load(input:proc_macro::TokenStream)->proc_macro::TokenStream{
    // let i:TokenStream = input.into();
    return quote!(
        Handle{
            id: todo!(),
            handle_type: todo!(),
            marker: std::marker::PhantomData,
        }
    ).into();
}

#[proc_macro]
pub fn time(input:proc_macro::TokenStream)->proc_macro::TokenStream{
    // let i:TokenStream = input.into();
    return quote!(
        bevy::prelude::Time::default()
    ).into();
}

#[proc_macro]
pub fn delta_seconds(input:proc_macro::TokenStream)->proc_macro::TokenStream{
    // let i:TokenStream = input.into();
    return quote!(
        (0.0)
    ).into();
}
#[proc_macro]
pub fn delta_time(input:proc_macro::TokenStream)->proc_macro::TokenStream{
    // let i:TokenStream = input.into();
    return quote!(
        0.0
    ).into();
}

#[proc_macro]
pub fn windows(input:proc_macro::TokenStream)->proc_macro::TokenStream{
    // let i:TokenStream = input.into();
    return quote!(
        bevy::prelude::Windows::default()
    ).into();
}

#[proc_macro]
pub fn internal_windows(input:proc_macro::TokenStream)->proc_macro::TokenStream{
    // let i:TokenStream = input.into();
    return quote!(
        windows
    ).into();
}


#[proc_macro]
pub fn resource(input:proc_macro::TokenStream)->proc_macro::TokenStream{
    let i:TokenStream = input.into();
    let (_,name) = is_mut_and_get_name(i);
    let t_id = to_ident(&name);
    return quote!(
        #t_id::default()
    ).into();
}
#[proc_macro]
pub fn res(input:proc_macro::TokenStream)->proc_macro::TokenStream{
    resource(input)
}

#[proc_macro]
pub fn internal_resource(input:proc_macro::TokenStream)->proc_macro::TokenStream{
    let i:TokenStream = input.into();
    let (imut,name)=  is_mut_and_get_name(i);
    let ident = to_ident(&format!("resource_{}{}",if imut{"mut_"}else{""},name.to_ascii_lowercase()));
    return quote!(
        #ident
    ).into();
}


 fn is_mut_and_get_name(s:TokenStream)->(bool,String){
    let mut iter = s.clone().into_iter();
    let t = iter.next().unwrap();
    if is_mut(&t) {
        (true,iter.collect::<TokenStream>().to_string())
    }else {
        (false,s.to_string())
    }
}

/*


Time{
        startup: todo!(),
        first_update: todo!(),
        last_update: todo!(),
        paused: todo!(),
        relative_speed: todo!(),
        delta: todo!(),
        delta_seconds: todo!(),
        delta_seconds_f64: todo!(),
        elapsed: todo!(),
        elapsed_seconds: todo!(),
        elapsed_seconds_f64: todo!(),
        raw_delta: todo!(),
        raw_delta_seconds: todo!(),
        raw_delta_seconds_f64: todo!(),
        raw_elapsed: todo!(),
        raw_elapsed_seconds: todo!(),
        raw_elapsed_seconds_f64: todo!(),
        wrap_period: todo!(),
        elapsed_wrapped: todo!(),
        elapsed_seconds_wrapped: todo!(),
        elapsed_seconds_wrapped_f64: todo!(),
        raw_elapsed_wrapped: todo!(),
        raw_elapsed_seconds_wrapped: todo!(),
        raw_elapsed_seconds_wrapped_f64: todo!(),
    }
 */


#[proc_macro]
pub fn auto_sys(input:proc_macro::TokenStream)->proc_macro::TokenStream{
    // let tokens:Vec<TokenTree> = vec![];
    let input_r:TokenStream = input.into();
    let mut stream = input_r.into_iter();
    let system_name = stream.next().unwrap();
    let args = stream.next().unwrap();
    let body = stream.next().unwrap();
    let arg_stream = transform_args(args);
    // dbg!(&arg_stream);/
    let ts = quote!{
        pub fn #system_name (#arg_stream) #body
    };

    // proc_macro::TokenStream::from_iter(ts.into_iter().collect())
    ts.into()
}


fn transform_args(body:TokenTree)->TokenStream{
    args(body).iter().map(|e|->TokenStream{
        match e {
            TokenTree::Ident(e)=>{
                match e.to_string().as_str() {
                    "Commands"=>quote!{
                        mut commands:Commands
                    },
                    _=>TokenTree::Ident(e.clone()).into()
                }
            },
            _=>e.clone().into()
        }
    }).flat_map(|e|e.into_iter().collect::<Vec<TokenTree>>()).collect()
}

fn args(body:TokenTree)->Vec<TokenTree>{
    match body {
        TokenTree::Group(e) => e.stream().into_iter().collect(),
        _=>vec![]
    }
}