mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
async fn copy_out_user_session_logs(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);

    let copied_csv = conn
        .copy_out_raw(
            "COPY (SELECT * FROM user_session_logs ORDER BY id) TO STDOUT WITH (FORMAT CSV, HEADER)",
        )
        .await
        .unwrap()
        .to_csv();

    let expected_csv = r#"
id,event_date,user_id,event_name,session_duration,page_views,revenue
1,2024-01-01,1,Login,300,5,20.00
2,2024-01-02,2,Purchase,450,8,150.50
3,2024-01-03,3,Logout,100,2,0.00
4,2024-01-04,4,Signup,200,3,0.00
5,2024-01-05,5,ViewProduct,350,6,30.75
6,2024-01-06,1,AddToCart,500,10,75.00
7,2024-01-07,2,RemoveFromCart,250,4,0.00
8,2024-01-08,3,Checkout,400,7,200.25
9,2024-01-09,4,Payment,550,11,300.00
10,2024-01-10,5,Review,600,9,50.00
11,2024-01-11,6,Login,320,3,0.00
12,2024-01-12,7,Purchase,480,7,125.30
13,2024-01-13,8,Logout,150,2,0.00
14,2024-01-14,9,Signup,240,4,0.00
15,2024-01-15,10,ViewProduct,360,5,45.00
16,2024-01-16,6,AddToCart,510,9,80.00
17,2024-01-17,7,RemoveFromCart,270,3,0.00
18,2024-01-18,8,Checkout,430,6,175.50
19,2024-01-19,9,Payment,560,12,250.00
20,2024-01-20,10,Review,610,10,60.00"#;

    assert_eq!(copied_csv.trim(), expected_csv.trim());
}

#[rstest]
async fn copy_out_research_project_arrays(mut conn: PgConnection) {
    ResearchProjectArraysTable::setup_parquet().execute(&mut conn);

    let copied_csv = conn
        .copy_out_raw(
            "COPY (SELECT * FROM research_project_arrays) TO STDOUT WITH (FORMAT CSV, HEADER)",
        )
        .await
        .unwrap()
        .to_csv();

    let expected_csv = r#"
experiment_flags,notes,keywords,short_descriptions,participant_ages,participant_ids,observation_counts,measurement_errors,precise_measurements
"{t,f,t}","{""Initial setup complete"",""Preliminary results promising""}","{""climate change"",""coral reefs""}","{""CRLRST    "",""OCEAN1    ""}","{28,34,29}","{101,102,103}","{150,120,130}","{0.02,0.03,0.015}","{1.5,1.6,1.7}"
"{f,t,f}","{""Need to re-evaluate methodology"",""Unexpected results in phase 2""}","{""sustainable farming"",""soil health""}","{""FARMEX    "",""SOILQ2    ""}","{22,27,32}","{201,202,203}","{160,140,135}","{0.025,0.02,0.01}","{2,2.1,2.2}""#;

    assert_eq!(copied_csv.trim(), expected_csv.trim());
}
