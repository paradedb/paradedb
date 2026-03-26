require "active_record"
require "parade_db"

ParadeDB::Arel::Visitor.install!
ParadeDB::Arel::Predications.install!

module RailsSnippetHarness
  DATABASE_URL = "postgresql://postgres:postgres@localhost:5422/postgres"

  class ApplicationRecord < ActiveRecord::Base
    self.abstract_class = true
  end

  class MockItem < ApplicationRecord
    include ParadeDB::Model

    self.table_name = "mock_items"

    has_many :orders,
      class_name: "RailsSnippetHarness::Order",
      foreign_key: :product_id,
      inverse_of: :mock_item
  end

  class Order < ApplicationRecord
    include ParadeDB::Model

    self.table_name = "orders"
    self.primary_key = "order_id"

    belongs_to :mock_item,
      class_name: "RailsSnippetHarness::MockItem",
      foreign_key: :product_id,
      inverse_of: :orders
  end

  module_function

  def establish_connection!
    ActiveRecord::Base.establish_connection(DATABASE_URL)
  end
end

RailsSnippetHarness.establish_connection!

MockItem = RailsSnippetHarness::MockItem
Order = RailsSnippetHarness::Order
