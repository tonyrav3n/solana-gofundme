pub extern crate anchor_lang;

anchor_lang::declare_program!(gofundme);

#[cfg(test)]
mod test_initialize_fundraiser;

#[cfg(test)]
mod test_donate;
