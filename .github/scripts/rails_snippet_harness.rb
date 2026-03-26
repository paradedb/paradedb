require "active_record"
require "parade_db"

ParadeDB::Arel::Visitor.install!
ParadeDB::Arel::Predications.install!

module RailsSnippetHarness
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
    ActiveRecord::Base.establish_connection(normalize_database_url(database_url))
  end

  def database_url
    ENV.fetch("DATABASE_URL", "postgresql://postgres:postgres@localhost:5432/postgres")
  end

  def normalize_database_url(url)
    return "postgresql://#{url.delete_prefix('postgres://')}" if url.start_with?("postgres://")

    url
  end
end

RailsSnippetHarness.establish_connection!

MockItem = RailsSnippetHarness::MockItem
Order = RailsSnippetHarness::Order
