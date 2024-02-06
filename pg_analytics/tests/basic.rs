mod fixtures;

use fixtures::*;
use rstest::*;

#[rstest]
fn basic_select(mut user_session_log_table: TableConnection<UserSessionLogsTable>) {
    let columns: UserSessionLogsTableVec =
        user_session_log_table.fetch_collect("SELECT * FROM user_session_logs ORDER BY id");

    // Check that the first ten ids are in order.
    let ids = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    assert_eq!(&columns.id[0..10], ids, "ids are in expected order");
    let event_names =
        "Login,Purchase,Logout,Signup,ViewProduct,AddToCart,RemoveFromCart,Checkout,Payment,Review";

    assert_eq!(
        &columns.event_name[0..10],
        event_names.split(",").collect::<Vec<_>>(),
        "event names are in expected order"
    );
}

#[rstest]
fn array(mut research_project_arrays_table: TableConnection<ResearchProjectArraysTable>) {
    let _columns: ResearchProjectArraysTableVec = research_project_arrays_table
        .fetch_collect("SELECT * FROM research_project_arrays ORDER BY notes");
}
